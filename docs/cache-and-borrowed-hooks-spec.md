# Spec: `@cache` field memoization + borrowed hooks (read returns & write args)

Status: design proposal (not implemented). Owner: codegen track.

Two orthogonal codegen features that, together, close the DNS-shaped performance
gap and a latent correctness bug. They are independent — either can land first —
but they compose.

- **`@cache(len | value)`** — opt-in per-field memoization. Attacks the *number*
  of times an expensive producer runs (the big lever).
- **Borrowed hooks** — let a hook touch packet/caller bytes by reference instead
  of by copy. Two mirror-image directions:
  - **read: borrowed *returns*** — a parse hook returns data borrowed from the
    packet instead of an owned copy. Attacks the *per-call cost* of the producer
    (the small lever), and fixes lossy string conversion.
  - **write: borrowed *args*** — a write hook accepts the value-to-encode by
    reference (a `&[u8]`, label slice, or lazy view) instead of only an owned
    `u64`. The dual of borrowed returns; required for writing
    hook-encoded *content* (DNS names, TLS/MQTT composed fields), not just
    hook-encoded *lengths*.

### Read path vs write path — which feature applies where

The performance pathology in §0 is a property of the **reader's lazy,
backward-walking offset model**. The writer does not share that model, so the two
features land asymmetrically:

| feature | read (parse) path | write (serialize) path |
|---|---|---|
| `@cache` offset/value memo | **needed** — fixes the O(n²) re-walk | **not needed** — see §1 "Write path"; lengths are eagerly known, offsets accumulate forward |
| borrowed hook **returns** | **needed** — hook returns `&'a`/lazy view from packet | n/a (a writer doesn't return parsed values) |
| borrowed hook **args** | n/a (a parser doesn't take a value to encode) | **needed** — write hook accepts borrowed value-to-encode; generalizes today's `u64`-length-only write hook |

So a writer never grows a `Cell`/`OnceCell` cache slot; the borrowed-type
*grammar* work in §2 is shared (same `path_to_tokens` extension serves both a
borrowed return type and a borrowed arg type).

---

## 0. Motivation (the measured problem)

binparse generates lazy zero-copy views: `struct Dns<'a> { data: &'a [u8] }`,
field getters are `&self -> ParseResult<T>` and compute their offset on demand by
walking `*_end_offset()` of prior fields. There is no per-instance storage, so
nothing is memoized.

For **fixed layouts** (UDP, TCP) every offset is a compile-time constant, the
compiler const-folds the `+`s, and there is nothing to cache — these are already
optimal (~5× of a hand-written slice reader, dominated by other factors).

For **variable-length fields** the offset is *not* a constant — it is the return
of a producer (a `@hook`, a VLA, a length expr). Concretely, generated code:

```rust
pub fn qname_end_offset(&self) -> Len {
    Len { byte: 12, .. } + match self.qname_raw() {   // qname_raw() RUNS dns_name() → allocates
        Ok((_, consumed)) => Len { byte: consumed, .. },
        Err(_) => Len::ZERO,
    }
}
```

The compiler **cannot** elide `qname_raw()` — it has an observable side effect
(allocation) and is data-dependent on the buffer. So every downstream field that
needs an offset re-runs it. Because `aname`'s offset depends on `qname`'s end, and
`aname` is itself a hook, the calls compound.

Measured (callgrind, Linux x86), one DNS extraction (`id + qname + A addr`):

| metric | value |
|---|---|
| `dns_name` hook invocations | **58** |
| heap alloc / free | 232 / 232 |
| instructions vs simple-dns | 19× |
| of which malloc/free | ~40% |

Of the 58 calls, only **~1** is an actual value read (`qname()`); the other **~57**
are *offset* re-derivations. That is the whole gap.

Timings (mac), same extraction:

| variant | time |
|---|---|
| binparse, hook returns `String` (join + `from_utf8_lossy`) | 4.67 µs |
| binparse, hook returns `Vec<Vec<u8>>` (owned labels, no UTF-8) | 3.23 µs |
| simple-dns, `Name::as_bytes()` (borrowed, zero-copy) | 0.12 µs |

The `String → Vec<Vec<u8>>` change (already shipped) is **1.45×** — it makes each
of the 58 calls cheaper but leaves the count untouched. Memoizing the count is the
**~29×** lever.

---

## 1. `@cache(len | value)`

### Why

1. **Kill the offset re-walk** (the 58×). Cache a variable-length field's
   consumed length so downstream offset computations don't re-run its producer.
2. **Kill value re-decode.** When a field's *value* is referenced by multiple
   downstream expressions and producing it is non-trivial, decode it once.

The second case is real in this repo: MQTT `@hook(leb128_unsigned, u64)
remaining_length` then `@len(remaining_length) body: union(...)`. Computing
`body`'s offset/length re-runs the leb128 hook each time `remaining_length` is
referenced. Generalizes to *any field whose parsed value feeds a downstream
`@len` / array-size / `union(discriminator)` expr where producing it costs more
than a load* (leb128, checksum-validated ints, length hooks).

### Two axes — and how they map to field kind

`len` (consumed length / end-offset) and `value` (the parsed `T`) are independent
because for most field kinds they are computed by *different* code paths:

| field kind | `len` cacheable? | `value` cacheable? | typically want |
|---|---|---|---|
| fixed int / bitfield | no (const-folded) | yes (a load) | usually neither |
| struct-ref | yes (sub-struct total length) | no (cheap slice view) | `@cache(len)` |
| array / VLA | yes (element walk / count) | no (cheap iterator) | `@cache(len)` |
| hook | coupled — one call yields both | coupled | `@cache(len, value)` → one slot |

### Syntax

```
@cache(len)            // memoize consumed length / end-offset
@cache(value)          // memoize the parsed value
@cache(len, value)     // both
@cache                 // shorthand for (len, value)
```

### Semantics & codegen

- **`len`** → add `Cell<Option<Len>>` to the view. `{field}_end_offset()` checks
  it, computes+stores on miss, returns the cached `Len` on hit. This is what
  collapses the 57 re-walks.
- **`value`, Copy `T`** (ints) → `Cell<Option<T>>`. Getter copies out of the cache;
  **signature unchanged** (`fn x(&self) -> ParseResult<T>`). This is the cleanest
  case — the MQTT/int example.
- **`value`, non-Copy `T`** (`Vec<…>`, `String`) → `OnceCell<T>`. Returning owned
  from cache would require cloning (defeats the cache), so the getter must return
  a **borrow**: `fn qname(&self) -> ParseResult<&[Vec<u8>]>`. This is an API
  change and must be accounted for (a `@cache(value)` on a heap field changes its
  getter's return type).
- **hook with `len, value`** → a single combined `OnceCell<(T, usize)>` slot,
  because a hook produces both in one call. `{field}()` and `{field}_end_offset()`
  both read from it. First offset-chain touch runs the hook once and caches
  `(value, consumed)`; the 57 re-walks and the eventual value read are then free.
- **Reference sites are transparent.** `@len(remaining_length)` already lowers to
  `self.remaining_length()`; if that getter consults the cache, every reference
  benefits with no extra plumbing.

### Construction

Cache slots are initialized empty wherever the view is built: `parse()`, nested
struct refs, array elements (each element has its own caches — correct, they are
independent), and union variant structs.

### Correctness

- **No invalidation needed.** A view's `data` is immutable; every field is a pure
  function of `data`, so a cached result can never be stale.
- **Never cache errors.** Getters return `ParseResult<T>`; the error path is cold.
  Cache only `Ok`, recompute on `Err`.
- **Fallible init gotcha.** Stable `OnceCell::get_or_init` is infallible
  (`get_or_try_init` is nightly). Emit the manual pattern instead:
  ```rust
  if let Some(v) = cell.get() { return Ok(/* use v */) }
  let v = self.{field}_raw()?;          // may early-return Err, uncached
  let _ = cell.set(v);                  // set() can't fail here (single-threaded, first write)
  Ok(/* use cell.get().unwrap() */)
  ```

### Type-capability impact

Adding `Cell`/`OnceCell` makes that view **`!Copy`** and **`!Sync`**. This is
**localized**: only structs that actually carry a `@cache`d (or auto-cached)
field get cache slots, so UDP/TCP/etc. stay `Copy`. If `Sync` views are required,
`len` can use `OnceLock`/atomics (heavier); the common single-threaded path should
default to `Cell`/`OnceCell`.

### Default policy — decision for the implementer

The offset-memo footgun argues for **auto-caching the end-offset of every
dynamic-offset field** (so users don't have to know to annotate around an O(n²)
blow-up). The counter-argument is that this silently makes those views `!Copy`.
Options, pick one:

- **Explicit only** (`@cache(len)` required): keeps views `Copy` by default;
  users must opt in. Pair with a **codegen lint/warning** when a dynamic field has
  downstream dependents and no `@cache(len)`.
- **Auto offset-memo** for dynamic fields, `@cache(value)` opt-in: fixes the
  footgun by default, accepts the `!Copy` consequence on affected structs only.

Recommendation: auto offset-memo for dynamic fields (it's almost always a win and
it's where the pathology lives), `value` opt-in.

### `dissect()`

`dissect()` returns an owned `FieldNode` tree and calls the per-field `&self`
methods internally; caches on that temporary view help within a single dissect
call. No special handling required — `hook_value` already maps non-numeric hook
returns to `Value::Opaque`.

### Acceptance targets

- DNS extraction: 58 → ~2 hook calls; ≈ 4.67 µs → ~0.16 µs (`String`) or ~0.06 µs
  (`Vec<Vec<u8>>`).
- MQTT: `remaining_length` leb128 decoded once per packet regardless of how many
  fields reference it.

### Write path: not applicable (and the structural reason)

`@cache` is a **read-path-only** feature. The writer does not get cache slots, and
none of §1's `Cell`/`OnceCell` machinery is emitted for `*Writer`/`*Content`
types. The reason is that the writer never reproduces the reader's pathology:

1. **Lengths are eagerly known, not lazily produced.** The reader derives a
   variable field's length by *running its producer* (a hook that allocates, a VLA
   walk) — that is the expensive thing worth memoizing. The writer is *given* every
   variable length up front: Mode 1 takes them as a `Lens` struct of ints; Mode 3
   infers them from the caller's content slice lengths. A byte region's length is
   `lens.field` — a load, const to the surrounding function, nothing to cache.

2. **Offsets accumulate forward, not by backward re-walk.** The reader computes
   field *N*'s offset as `end_offset(N-1)`, recursively, re-running every prior
   producer — O(n²). The writer's bulk paths (`write_into` / `to_vec` / the union
   variant writers) emit fields in order threading a **single running offset
   accumulator**, so each field's length is computed exactly once per write — O(n),
   re-running nothing. *This is a hard design rule for the dynamic-offset
   generalization: thread a forward offset; never lower a writer offset as a
   recursive backward chain.*

3. **Derived fields are computed once.** The write-path analogue of the MQTT
   `remaining_length` re-decode does not exist: `remaining_length` is *derived*
   (from the body's `encoded_len`), computed once during forward accumulation, and
   written. There is no reference site that re-decodes it.

**The one honest caveat (a benign, non-blocking micro-opt).** Mode 1's standalone
`*_mut()` accessors are independent function calls, so an accessor for a field that
sits *after* a dynamic field recomputes the prefix sum from the `Lens` on each
call — there is no shared accumulator across separate calls. This is the same
*shape* as the reader's re-walk, but not the same *magnitude*: the per-length cost
is a `lens` load or a cheap `width_fn` (leb128 width = a few iterations), never an
allocating producer, and accessors are normally called once each. It is therefore
**not** a parity requirement and **not** in scope. If a future profile ever shows a
hot Mode-1 accessor sitting past a large array-of-structs (where the recomputed
length is a real per-element `encoded_len` loop), the *same* `Cell<Option<Len>>`
offset-memo from §1 can be reused as a writer-local, opt-in micro-opt — but it is
explicitly deferred until measured.

---

## 2. Borrowed hooks: read returns and write args

A hook is a user-supplied codec for a field the generator can't lower itself. The
two directions are mirror images and share one piece of grammar (§2.4):

| | read hook (parse) | write hook (serialize) |
|---|---|---|
| target signature | `fn(&[u8], HookContext) -> ParseResult<(T, usize)>` | `fn(V, &mut [u8], WriteHookContext) -> WriteResult<usize>` |
| value | **returns** `T` | **accepts** `V` |
| borrowing wanted | `T` borrows from the **packet** (`&'a`) | `V` borrows from the **caller's content** (`&'a`) |
| count | returns `consumed` | returns `written` |
| context | `HookContext { offset, enclosing: &'a [u8] }` (bytes already parsed) | `WriteHookContext { offset, written: &[u8] }` (bytes already written) |

The write hook is the exact symmetric dual: read takes *bytes + ctx → (value,
consumed)*; write takes *value + mut-bytes + ctx → written*. One required function,
not two — see "Why one function" below.

### Read direction: borrowed returns

### Why

After memoization the residual cost is the **per-call allocation** inside the
producer. A hook that returns owned data (`String`, `Vec<Vec<u8>>`) copies the
bytes out of the packet. Letting it return **borrowed** data removes that copy,
and — for DNS specifically — also fixes a correctness bug: the current
`from_utf8_lossy` path silently replaces non-UTF-8 label bytes with U+FFFD, which
is wrong (DNS labels are opaque octets; DNS-SD names are UTF-8, others can be any
byte). Borrowing the raw bytes is both faster and lossless.

This attacks the *per-call cost* (the ~1.45×); memoization attacks the *count*
(the ~29×). They compose to simple-dns parity.

### Current limitation (the "why not today")

`@hook(fn, ReturnType)` parses `ReturnType` via `attr.rs::path_to_tokens`, which
only accepts `ast::Expr::Path` — a plain dotted identifier path:

```rust
fn path_to_tokens(expr: &ast::Expr<'_>) -> Result<TokenStream, Error> {
    match expr {
        ast::Expr::Path(segments) => { /* idents joined by :: */ }
        _ => Err(Error::InvalidHookArg),
    }
}
```

So anything with `&`, `<>`, `[]`, or a lifetime — `Vec<&[u8]>`, `&[u8]`,
`Cow<[u8]>`, `NameRef<'a>` — fails to parse. A hook can only return an owned,
path-nameable type. (That's why `dns_name` returns `String`, and why the
`Vec<Vec<u8>>` version needs the `DnsLabels` type alias to be path-nameable.)

### How to support it

1. **Grammar.** For the `@hook` return-type argument, accept a full Rust *type*
   token (references, generics, slices, lifetimes) instead of just an ident path.
   Either extend the attribute-arg grammar with a "type" arg kind, or capture a
   raw type token for this position.
2. **Lifetime threading.** The getter is emitted inside `impl<'a> Struct<'a>`, so
   `'a` is in scope. Two ways to get it into the return type:
   - **Explicit (simplest):** require the spec to spell the lifetime, e.g.
     `@hook(dns_name, Vec<&'a [u8]>)` or `@hook(dns_name, NameRef<'a>)`. Emit the
     type verbatim; `'a` resolves to the struct lifetime.
   - **Sentinel:** let the spec use a placeholder lifetime the codegen rewrites to
     the struct's `'a`.
3. **Borrow source already exists — no infra change.** `HookContext<'a>` carries
   `enclosing: &'a [u8]`, and the generated call already passes
   `enclosing: self.data` (which is `&'a`). A hook returning data borrowed from
   `ctx.enclosing` with `'a` works as-is:
   ```rust
   fn dns_name<'a>(_data: &[u8], ctx: HookContext<'a>) -> ParseResult<(NameRef<'a>, usize)>
   ```
4. **Dissect already handles it.** `field.rs::hook_value` maps any non-numeric
   return to `Value::Opaque` (binding `_`), so a borrowed return doesn't break
   tree generation.

### Spectrum of return types

- `Vec<&'a [u8]>` — borrows the label bytes, still allocates the spine `Vec`.
- **Best:** a lazy `NameRef<'a>` view that holds `enclosing + offset` and yields
  labels on demand (an iterator), allocating **nothing**. The consumer stringifies
  only if they need to. This is the true analogue of simple-dns's `Name::as_bytes()`.

### Interaction with caching

Caching a borrowed value is fine: `OnceCell<NameRef<'a>>` borrows `'a`, which the
view already has. With `@cache(len)` (so the hook runs ~once) + a zero-alloc
`NameRef<'a>` return, the DNS path has essentially no heap traffic.

### Acceptance target

DNS hook allocation-free; combined with offset-memo, DNS extraction ≈ simple-dns
(~0.12 µs), with lossless (non-UTF-8-safe) labels.

### Write direction: borrowed args

#### Why

Today's `@write_hook(encode_fn, width_fn)` only inverts a hook-encoded **length**:
the value handed to the encoder is always the *derived* `u64` length of a trailing
region (see `DynamicTailHook` in `writer.rs`; runtime `write_leb128_unsigned`).
That is enough for MQTT's `remaining_length` but not for any format whose hook
encodes **content** rather than a length — a DNS name, a TLS/MQTT composed field, a
zig-zag-encoded signed value carried as a real field. Those are exactly the
writer's open gaps (mqtt/dns/tls). To write them, the encoder must accept the
field's *value*, and that value is frequently **borrowed** from the caller (a
`&[u8]`, a `&[&[u8]]` of labels, a lazy `NameRef` view) — the precise dual of a
read hook returning borrowed data from the packet.

#### Generalized write-hook shape — one required function

**Decision (settled with user): the write hook is ONE required function.** A second
`width` predictor exists only as an *optional accelerator* (see "Sizing" below), not
as part of the core contract.

```rust
// V is the field's value type; may borrow from the caller (e.g. &'a [u8]).
// ctx exposes the destination prefix already written (for back-referencing codecs);
// ignored as `_ctx` by simple hooks, exactly as read hooks ignore their HookContext.
fn encode(value: V, dst: &mut [u8], ctx: WriteHookContext) -> WriteResult<usize>;
// WriteHookContext { offset: usize, written: &[u8] }  // dst[..offset] already filled
```

This is the exact dual of the read hook and is literally "value + mut-buffer in,
size out." The current `encode_fn(u64, &mut [u8]) -> _` length path is the special
case `V = u64` with `ctx` ignored; it keeps working unchanged (its optional `width`
is today's `width_fn`).

**Why one function, not two.** A separate `width(value)` only buys the ability to
know a field's byte-length *without writing it* — needed for (a) exact buffer
pre-sizing and (b) placing a field at a fixed offset before its predecessors are
written (Mode-1 random access / length-prefix math). A left-to-right write pass
needs neither: each `encode` *returns* its actual width, so the next field's cursor
is just "where the last write ended." And the powerful case below (compression)
makes `width` impossible to provide anyway — a compressed length depends on what is
already in the buffer, which `width(value)` can't see. So the core contract is one
function; `width` is a pure optimization, demoted to optional.

**Two value sources, mirroring derived-vs-content fields:**

- **derived field** (length/discriminant): `V = u64`, value computed by the writer
  (today's behaviour). No setter, not in `Content`.
- **content field** (hook-encoded payload): `V` is the field's content type, taken
  from the caller's `Content`/setter. This is where borrowing matters — `V` may be
  `&'a [u8]`, `&'a [&'a [u8]]`, `NameRef<'a>`, etc., borrowing from the same `'a`
  the `Content` struct already carries.

#### Lifetime threading

Symmetric to §2's read story: the read getter is emitted inside `impl<'a>
Struct<'a>` so `'a` is the **packet** lifetime; the write encoder is invoked inside
`write_into<'a>(dst, content: &Content<'a>)` so `'a` is the **caller/content**
lifetime. A borrowed `V` spelled `&'a [u8]` / `NameRef<'a>` resolves against that
content `'a` with no new infrastructure — `Content` already borrows its byte
regions.

#### Back-referencing formats (compression) — chosen: context-carrying (B)

**Decision (settled with user): support the powerful, context-carrying model.** The
`WriteHookContext { offset, written: &[u8] }` above is part of the contract. To emit
a compressed name the encoder scans `ctx.written` for a reusable suffix and emits a
pointer — the write-side analogue of the read `HookContext { offset, enclosing }`.

The reason this is a real decision and not free: a compressing encoder has a
**content- and position-dependent width** (the same name costs 2 bytes compressed
vs N uncompressed depending on what precedes it), so it cannot be predicted before
writing. That rules out the "predict every width, sum into `encoded_len`, then write
into a perfectly-sized buffer" model for any struct containing such a hook. The
writer switches those structs to **write-then-measure**:

- **`to_vec`** writes into a growable `Vec<u8>` — no pre-size needed, the actual
  bytes accumulate. Simplest and always correct.
- **`write_into(&mut [u8])`** writes left-to-right into the caller's buffer and
  returns the **actual** number of bytes written; `encoded_len` provides an
  **upper bound** (worst-case / uncompressed) for the caller to allocate, who then
  truncates to the returned length. (Standard `max_encoded_len` + actual-written
  serialization shape.)

**Consequence — localized, not a regression:** a field positioned *after* a
context-carrying hook field is **sequential-write-only** — its start offset depends
on the not-yet-known compressed width of the earlier field, so no fixed-offset
`*_mut()` accessor can be handed out for it. Fields *before* the first compressing
hook, and any struct with none, keep full Mode-1 random access. Non-compressing
dynamic fields (byte regions, leb128 lengths — widths predictable from the `Lens`)
also keep Mode-1, because their offsets are computable without writing. The current
writer is dynamic-at-tail-only, so nothing that works today regresses.

#### Sizing — the optional `width` accelerator

`encoded_len` and Mode-1 random access want a field's size *without* writing it.
Provide that as an **optional** second hook fn `width(value: V) -> usize`:

- **present** (predictable-width codecs: leb128, zig-zag, escaping, base64) → the
  generator keeps **exact** `encoded_len` and full Mode-1 random access for that
  field and everything after it. Avoids a wasteful trial-encode just to size large
  predictable content.
- **absent** (compressing hooks, or just unspecified) → fall back to
  write-then-measure: `to_vec` grows, `write_into` uses the upper-bound +
  actual-written contract, and following fields go sequential-only.

So `@write_hook(encode_fn)` is the minimal form; `@write_hook(encode_fn, width_fn)`
opts into exact sizing. Today's length path is the latter and is unchanged.

#### Grammar & DSL surface

Reuses §2.4's `path_to_tokens` extension verbatim — a borrowed *arg* type needs the
same `&`/`<>`/`[]`/lifetime support as a borrowed *return* type. The field's value
type is taken from the field declaration (the same type the read `@hook(fn, T)`
already spells), so `@write_hook` names only fn paths, not the value type. A missing
`@write_hook` where one is required stays the hard `writer::Error::MissingWriteHook`
it is today.

#### What it unblocks

DNS / TLS / MQTT *content* writing (the composed-hook gaps in the writer memo), and
any field carrying a `@hook`-decoded non-length value. Combined with the
dynamic-offset generalization (offsets after a hook-encoded field) it removes the
last hook-shaped blocker on writer/parser parity.

#### Acceptance target

A `@hook`-decoded **content** field round-trips: `parse → value`, `value → write`,
re-`parse` yields the same value, for at least one borrowed-arg hook (e.g. a
`&[u8]`-input encoder). When the hook provides the optional `width`, it must agree
with `encode`'s written count under fuzz. A context-carrying (compressing) hook
round-trips through the write-then-measure path: `write_into`'s returned length
matches the bytes a re-`parse` consumes, and `encoded_len`'s upper bound is ≥ that.

---

## 3. How the two compose

On the **read** path the two levers stack to close the DNS gap:

| lever | mechanism | DNS effect |
|---|---|---|
| call **count** | `@cache(len)` offset memo | 58 → ~2 hook calls (~29×) |
| per-call **cost** | borrowed/lazy hook return | removes the residual alloc (~1.45×+) |

Fixed-layout structs const-fold their offsets, so neither read feature applies to
them and there is no regression risk there.

On the **write** path the only relevant lever is borrowed hook *args* (§2 write
direction); `@cache` is structurally unnecessary there (§1 "Write path"). The
shared grammar work (§2.4) lands once and serves both directions, so the natural
ordering is: extend the type grammar first, then read borrowed returns and write
borrowed args fall out of it independently.

---

## 4. Code touchpoints (for the implementer)

- `binparse-codegen/src/attr.rs` — `parse_hook` (~326), `path_to_tokens` (~339):
  grammar extension for borrowed return types; parse `@cache(...)` args.
- `binparse-codegen/src/field.rs` — `generate_vla_hook` (~849), `generate_fixed_hook`,
  `hook_value`/`hook_value_binding` (~1066/1078): emit cache slots + cached
  getters; thread `'a` into the return type.
- `binparse-codegen/src/struct_.rs` — view struct definition + `parse()`
  constructor: add cache-slot fields and initialize them.
- `binparse-codegen/src/expr.rs` — reference lowering already routes through the
  value getter; confirm cached getters are used at reference sites.
- `binparse/src/lib.rs` — `HookContext<'a>` (already carries `enclosing: &'a [u8]`);
  no change needed for borrowing. Add the `WriteHookContext { offset, written: &[u8] }`
  mirror for write hooks (chosen model B).

### Write-path touchpoints (borrowed args)

- `binparse-codegen/src/writer.rs` — `parse_write_hook` (~653, make `width_fn`
  optional) and `DynamicTailHook` (~60): generalize the encoder value type from the
  hardwired `u64` length to the field's value type; for a **content** hook, source
  `V` from the `Content`/setter instead of the derived length. Thread the content
  `'a` into a borrowed `V`. No `Cell`/cache slots. Two emit paths by hook kind: a
  `width`-providing hook keeps the forward offset accumulator + exact `encoded_len`
  (§1 "Write path"); a context-carrying hook switches its struct to write-then-measure
  (`to_vec` grows, `write_into` returns actual + `encoded_len` upper-bounds), and the
  fields after it become sequential-write-only (no `*_mut()`).
- `binparse-codegen/src/attr.rs` — same `path_to_tokens` extension as the read
  side; the write hook reads its fn paths from raw attrs already
  (`ParsedAttrs` ignores `write_hook`), so only the *type* grammar is shared.
- `binparse/src/hooks.rs` — `write_leb128_unsigned` is the existing `u64` encoder;
  add borrowed-arg companions (e.g. a `&[u8]`/label-slice encoder + its `width`)
  as the round-trip duals of the read hooks.

## 5. Open decisions

1. Offset-memo: auto for dynamic fields, or explicit `@cache(len)` only? (footgun
   vs `Copy` preservation; consider a lint for the explicit route).
2. `Sync` views: `Cell`/`OnceCell` (`!Sync`) default, or `OnceLock` when a
   `Sync` view is required?
3. Lifetime spelling for borrowed returns/args: explicit `'a` in the spec, or a
   codegen-rewritten sentinel? (Decide once; applies to both directions.)
4. `@cache(value)` on non-Copy types changes the getter to return `&T` — accept
   the signature change, or restrict value-caching to `Copy` types?
5. Bare `@cache` default = `(len, value)`? And does `@cache(len, value)` on a hook
   collapse to one combined slot (recommended yes)?
6. Confirm `@cache` is read-path-only and the writer stays cache-free (§1 "Write
   path"). Recommended **yes**; the Mode-1 accessor caveat is deferred until
   measured.
7. Should `@write_hook` re-use the read field's value type implicitly (recommended
   yes), or allow a separately-spelled write value type for codecs whose owned
   write input differs from the borrowed read output?
8. **Write-hook context — RESOLVED: context-carrying (B).** The contract is the
   single `encode(value, dst, ctx: WriteHookContext) -> written`; `width` is an
   optional sizing accelerator. Structs with a context-carrying hook use
   write-then-measure (`to_vec` grows; `write_into` sizes by upper bound and returns
   actual written), and fields after such a hook are sequential-write-only. See §2
   "Back-referencing formats" and "Sizing." (Was a fork between context-free (A) and
   context-carrying (B); user chose the more powerful B.)
