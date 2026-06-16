# Eager Parse (`parse_full`) — Design + Next-Session Handoff

A second generated read API that does **one forward pass** and materializes every
field into an **immutable `*Parsed<'a>` view** (plain fields, `&self` getters), for
"read-everything" consumers: dissectors / Wireshark-style tools, full validation,
indexing, packet→owned conversion. The existing lazy `parse()` (pay-per-field,
`&mut self`) stays exactly as-is for routing / filtering / partial reads. Written
2026-06-14.

---

## TL;DR — what to build

Add, alongside the current `parse()`:

```rust
impl<'a> Packet<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), ::bytesmith::ParseError>;       // lazy, unchanged
    pub fn parse_full(data: &'a [u8]) -> Result<(PacketParsed<'a>, &'a [u8]), ::bytesmith::ParseError>; // NEW: eager
}
```

`PacketParsed<'a>` stores **decoded values** (plain `T`, not `Option<Cache>`), one
**bit-range per field**, and exposes **`&self`** getters. It implements `Dissect`
with a `field_tree()` built from stored values — no re-parse, no hook re-runs.

Build it as a **parallel generator** (don't mutate the lazy generator). Phased plan
below. Do **not** replace lazy parsing.

---

## Why — measured, not hypothetical

The lazy path already memoizes via `@cache(len|value)` (landed: hooks
`field.rs:935-1238`, unions `union_.rs:341-401`; bare `@cache` = len+value,
`attr.rs:226-235`). But memoization is a *bet*: it only pays off when
`recompute_cost × reuses > storage_bytes × cursor_churn`. For a **cheap-to-recompute**
field that inequality flips and the cache becomes a pessimization.

A union body is exactly that cheap case: the variant structs are themselves lazy
borrowed views, so "selecting" a union is a match + a slice-wrap + a variant bounds
check — nearly free. But `@cache(value)` on it stores
`Option<ParseResult<(enum, rest)>>` (~72–80 B) **inside the cursor**, which
`parse()` then returns **by value** — so every parse moves a struct that's 2–3×
bigger, and locality suffers for all other field accesses.

**Benchmark (verified).** Isolated throwaway clone, a custom bytesmith-only
`mqtt_cache` bench (no rumqtt deps), single-core-pinned (`taskset`), quiet 16-core
Zen4 Linux box. Toggled **only** `@cache` on the `body:` union in
`mqtt_v3.bsm`/`mqtt_v5.bsm` (kept it on the varint `remaining_length`/`prop_len`):

| group | `parse()` with `@cache` | `parse()` no body-cache | Δ |
|---|---|---|---|
| v3 CONNECT | 34.7 ns | 19.1 ns | **−44%** |
| v3 PUBLISH | 32.8 ns | 17.7 ns | **−45%** |
| v5 CONNACK | 40.8 ns | 29.2 ns | **−28%** |

`full_dissect` (touch every field) moved only within noise (≤4%, and a *same-config*
rerun drifted ~5–10% on this allocation-heavy arm under the `powersave` governor —
so treat its deltas as no-effect). The `parse` deltas, by contrast, were stable to
<2% on a same-config rerun → **trustworthy**.

**Read this twice:** caching a cheap union nearly **doubles** `parse()` and gives
**no** measurable benefit even when you read every field. That is the whole case for
eager parse: stop trying to memoize into the cursor. Materialize once into a lean,
immutable, separate struct — plain `T` (error already lifted to the top-level
`Result`), no `Option`/`Result` wrapper, no `&mut self`, no cursor bloat. Eager is
"compute once" *without* the cache tax.

Secondary motivation: the lazy read path is now **`&mut self` on every getter**
(`field.rs:250,266,287`; `Dissect` trait `lib.rs:27-29`) — even read-only access
needs a `mut` binding and can't be shared `&self`. Eager's immutable `*Parsed`
view fixes that wart for free.

---

## Companion quick win (separable — can land before/without eager)

**Drop `@cache` from the `body:` union** in `bytesmith-protocols/specs/mqtt_v3.bsm:5`
and `mqtt_v5.bsm:5` (keep it on `remaining_length` and `prop_len` — varints *are* the
expensive-recompute case that caching wins). Measured ~2× faster `parse()`.

Caveat: this changes `body()`'s return from `&mut MqttPacket_body` to owned
`MqttPacket_body`, so update the by-value match bindings in
`bytesmith-bench/benches/mqtt.rs` (the arms that do `MqttPacket_body::Connect(c) =>
c.keep_alive()` will need `mut c` once `body()` returns owned), then run the GATE.
Pure memoization change — no behavior/wire change.

---

## Scope / non-goals (calibrate effort honestly)

- **Do not** replace `parse()`. Eager is additive.
- Biggest real wins over "lazy + cache": **arrays of structs** (which `@cache`
  cannot cover at all — it's hook/union-only, and `array.rs:413-417,550-556` rebuild
  the iterator + re-`parse` every element on each `field_tree`) and the full
  **`field_tree`/dissection** path.
- Fixed primitive headers (Ethernet/UDP/fixed TCP): eager is ~neutral — lazy getters
  already compile to direct slice reads. Don't promise speedups there.
- **Before committing to the full parallel-codegen build, weigh the cheaper
  alternative:** extend `@cache(value)` to struct-arrays (`Option<Vec<ElemParsed>>`)
  — closing the one gap caching misses — plus the companion quick win above. That may
  capture most of the value at a fraction of the surface. The full eager build is
  justified primarily by (a) the immutable `&self` API as a *product feature* and
  (b) one-pass full dissection. If those aren't wanted, prefer the array-cache route.

---

## Design

### `parse_full` contract
One forward pass that, for valid input: returns the **same** consumed length / `rest`
and the **same** `ParseError`s as `parse()`; never panics on truncated/malformed
input; decodes every field on the chosen conditional/union path; runs each hook
once; selects each union once; traverses each array once; stores one bit-range per
field; keeps zero-copy borrows where practical.

### Generated parsed type
```rust
pub struct PacketParsed<'a> {
    data: &'a [u8],
    len: ::bytesmith::Len,
    // one decoded member + one ::core::ops::Range<usize> bit-range per field
}
impl<'a> PacketParsed<'a> {
    pub fn len(&self) -> ::bytesmith::Len;
    pub fn ttl(&self) -> u8;                       // Copy: by value
    pub fn qname(&self) -> &NameRef<'a>;           // non-Copy: by ref
    pub fn options(&self) -> Option<&'a [u8]>;     // conditional
    pub fn chunks(&self) -> &[SctpChunkParsed<'a>]; // materialized array
    pub fn field_bit_range(&self) -> ::core::ops::Range<usize>;
}
```
Eager getters are mostly **infallible** (`parse_full` already did + validated the
work) and **`&self`** — deliberately *not* the lazy getter signatures. The eager
type is a distinct generated type. (`@error` union variants may still surface as
`Result<…, Error>` to preserve current semantics.)

### Storage policy
| DSL field kind | Parsed storage |
|---|---|
| `u8`/`u16`/…/signed | `T` |
| `b<N>` | `u8` |
| `[u8; expr]`, `@greedy [u8]`, `@payload [u8]` | `&'a [u8]` |
| `[u16; expr]` etc. (multi-byte prim array) | `&'a [u8]` + on-demand decode, **or** `Vec<T>` (decide; prefer slice+iter to avoid alloc) |
| `[Struct; expr]` / greedy struct array | `Vec<StructParsed<'a>>`  ← the array win |
| `StructRef` | `StructParsed<'a>` |
| `@len(expr) StructRef`/`union` | parsed + `rest: &'a [u8]` |
| `union(...)` | **lean** generated parsed enum (no `Option`/`Result` wrapper) |
| hook → `T` | `T` (store consumed length too) |
| conditional field | `Option<T>` |
| skipped field | store privately iff later exprs/tree need it; no pub getter |

Store ranges as **bit** ranges (`Range<usize>`) — enough for `field_tree`; add
`start/end: Len` only if you want `*_start_offset()` on `Parsed`.

### `Dissect` from stored data
`PacketParsed::field_tree()` builds nodes from stored values + stored ranges +
stored union variant + stored array elements. It must **never** call lazy
`Packet::parse()` or re-run hooks. Implement `Dissect<'a>` for the parsed type
(trait is `&mut self` today — `struct_.rs:343-351` — the impl can ignore the mut).

### Expression lowering
`expr::lower` currently emits field refs as `self.#field()` (lazy `&mut` getters,
`expr.rs`). Eager needs a **second mode** (`expr::lower_full`) that resolves a field
ref to the **local variable / stored member** in the forward pass — required for
`[u8; length - 8]`, `@len(remaining_length) … union(packet_type)`, `if (ihl > 5)`.

### Bounds-check discipline (unchanged from lazy)
Never index before a length check. Mirror `field.rs:435-446`:
```rust
let need = offset.saturating_add(N);
if data.len() < need { return Err(ParseError::NotEnoughData { expected: need, got: data.len() }); }
let v = u16::from_be_bytes(data[offset..need].try_into().unwrap()); // safe only after the check
offset = need;
```

---

## Plan (phased; each phase ends green at the GATE)

1. **Parsed type + primitives.** `PacketParsed<'a>`, `parse_full`, for primitives,
   bitfields (carry a `bit_offset`), fixed/`expr`-sized `[u8]`, simple validation,
   `*_bit_range()`. *Accept:* compiles for simple specs; values equal lazy getters;
   `rest` equals lazy `parse()`; truncation → `NotEnoughData`, no panic.
2. **Hooks + dynamic offsets.** var-len consuming hooks + fixed hooks; store
   `(value, consumed)`; bounded `@len` windows + `rest`. *Accept:* a counter hook
   fires **once** under `parse_full` + parsed `field_tree()`; MQTT `remaining_length`
   decoded once; DNS name hook once per name.
3. **Struct refs + unions.** `StructRef → InnerParsed`; bounded variants + `rest`;
   **lean** parsed union enum (no `Option`/`Result` in storage); `@error` behavior.
   *Accept:* union selected once; parsed `field_tree()` doesn't call the lazy union
   getter; existing union validation errors preserved. **This is the phase that
   beats `@cache`** — verify it actually does (re-run the `mqtt_cache` A/B style).
4. **Arrays.** counted prim arrays; `@greedy`/`@until` byte arrays as borrowed
   slices; `[Struct; n]`/greedy struct arrays → `Vec<ElemParsed>` (reuse `@max_iter`
   guard `array.rs:386-411`). *Accept:* TCP/DHCP/SCTP option/chunk arrays parse once;
   parsed `field_tree()` reads stored elems; `@max_iter` errors match lazy. *Caveat:*
   `Vec` trades the zero-copy iterator's no-alloc for one allocation — net win only
   when the array is read >~twice; say so, don't silently regress single-pass.
5. **Parsed `field_tree()` + `handoff()`** from stored values/ranges/variants/elems.
   *Accept:* node values/ranges match lazy `field_tree` for valid packets;
   hidden/skip/pad/absent nodes match; handoff uses stored discriminators + payload
   slice.

---

## Where to generate — file map (read path; `grep` names, **lines drift**)

Architecture: `lib.rs → struct_.rs → field.rs → type_/mod.rs → type_/*.rs`.
`use bytesmith_dsl as ast;` everywhere. Start by generating `parse_full` + the parsed
type inside `struct_::generate_struct` after the lazy tokens; split into
`full.rs`/`full/field.rs`/`full/type_.rs` if it grows.

| file | approx | role / where to hook |
|---|---|---|
| `struct_.rs` | `generate_struct` :131, `parse_fn` :354, `field_tree` :214, `dissect` :305, `handoff` :311, `GeneratedStruct` :56, `StructAccum` :29 | emit `XParsed` + `parse_full` here; add a `FullStructAccum` cursor |
| `field.rs` | `generate` :79, offset getters (all `&mut self`) :244-295, `generate_validations` :551, hook value-cache :935/:1339 | per-field eager statement + stored member + `&self` getter |
| `type_/mod.rs` | dispatch :1, `GeneratedTypeInfo` | parallel eager dispatch |
| `type_/primitive.rs` `bitfield.rs` `array.rs` (`generate` :35, `tree_node` :492) `struct_ref.rs` `union_.rs` (`generate` :42, value-cache :341) `concat.rs` | per-type eager emitters |
| `expr.rs` | `lower` | add `lower_full` (field ref → local/stored, not `self.#f()`) |
| runtime `bytesmith/src/lib.rs` | `Len` :33, `ParseError` :84 (`NotEnoughData{expected,got}` :86), `HookContext` :76, `ParseResult` :106, `Dissect` :27; `FieldNode`/`Value`/`Status` in `tree.rs` | author sketches' field names are correct against these |

---

## GATE — run after EVERY codegen change (non-negotiable)

`build.rs` runs codegen for all 15 protocols, so a read-codegen regression breaks the
protocol crate too.

1. `cargo run -p bytesmith-codegen --example test`     (codegen smoke)
2. `cargo test  -p bytesmith-codegen`                  (golden/runtime tests)
3. `cargo build -p bytesmith-protocols --features all` (build.rs codegen, all 15)
4. `cargo test  -p bytesmith-protocols --features all` (smoke + round-trips + dissect)
5. `cargo clippy --all-targets`

Goldens: several tests `assert_generated_eq!` against pinned generated code; adding
`parse_full` will need new goldens (or regenerate). New eager behavior gets its own
tests in `bytesmith-codegen/src/tests/` + `bytesmith-protocols/tests/`.

---

## In-flight / don't collide

- **Another agent owns the writer codegen** (`bytesmith-codegen/src/writer.rs`, a
  separate pass). Eager read work is mostly orthogonal, but both touch `struct_.rs`
  and `field.rs` — coordinate, re-grep line numbers before editing.
- The `@cache(value)` work has **landed** (hooks + unions, `&mut self` accessors,
  plain `Option<T>` caches). Do **not** design against the older `Cell<…>` / `&self`
  world an earlier external sketch assumed.
- Never reintroduce a work-machine hostname or path into code/docs/commits (history
  was scrubbed of one). Benchmarks: report numbers + machine *class* only.

---

## Reproduce the benchmark

Custom `bytesmith-only` bench (no rumqtt deps → fast iterate). In a throwaway clone:
add `bytesmith-bench/benches/mqtt_cache.rs` with two arms per packet — `parse`
(`MqttPacket::parse(..)` + `black_box`) and `full_dissect` (`MqttPacket::dissect(..)`),
register `[[bench]] name="mqtt_cache" harness=false`. Then:
`taskset -c <core> cargo bench --bench mqtt_cache -- --save-baseline withcache`;
`sed` out `@cache` from the two `body:` unions; rerun with `--baseline withcache`.
Pin to one core; the parse deltas are stable, the dissect arm is noisy under
`powersave` (set the `performance` governor for clean full-read numbers).
