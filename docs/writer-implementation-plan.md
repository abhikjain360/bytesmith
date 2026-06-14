# Writer + perf implementation plan (handoff)

Status: roadmap. Owner: codegen track. Companion: `docs/cache-and-borrowed-hooks-spec.md`
(perf + write-hook contract design — read it first for §§ referenced below).

This doc is a self-contained execution plan so a fresh session can continue the
writer/parser-parity work and the perf work **without re-deriving any design**. All
the design decisions are settled; what remains is sequenced implementation.

---

## 0. Where things stand (git + test surface)

- **Branch:** `writer-codegen`. Writer subsystem committed at `d7d7fb7`
  ("Add zero-copy packet writer codegen subsystem", 20 files).
- **Uncommitted in the working tree but NOT ours:** `Cargo.lock`, `Cargo.toml`,
  `binparse-bench/benches/dns.rs`, `binparse-protocols/specs/dns.bp`,
  `binparse-protocols/src/hooks.rs`, `binparse-protocols/tests/smoke.rs`,
  `dns-profile/` — these belong to the **concurrent benchmark/DNS agent**. Do not
  touch, stage, or commit them. Our work stays additive and out of those files.
- **`docs/` is untracked** — this plan + the spec live on disk; commit them to the
  branch if you want them in history (they're our work; safe to `git add docs/`).
- **Writer test surface already in place** (mirror of every parser test type — keep
  all green after each slice):
  - `binparse-codegen/tests/writer_runtime.rs` — round-trips through the real reader.
  - `binparse-codegen/src/tests/writer_snapshots.rs` + `snapshots/writer_*.txt`.
  - `binparse-codegen/tests/writer_protocol_suite.rs` — real packets, pinned bytes.
  - `fuzz/fuzz_targets/generated_writers.rs` — "every successful write re-parses".

## 1. Hard constraints (apply to every slice)

1. **Opt-in & additive.** Writers emit only via `CodeGen::generate_writers`;
   `CodeGen::generate` (reader path) output stays **byte-for-byte unchanged** so the
   exact-match reader snapshots under `src/tests/snapshots/` never churn. Unsupported
   struct shapes **skip silently** (emit nothing, never panic) so every spec keeps
   generating its reader.
2. **Stay out of the reader hot path files** unless a slice genuinely requires it:
   avoid editing `struct_.rs` / `field.rs` / `type_/*` for *writer* work (they're the
   reader path and the bench agent's territory). Writer logic lives in `writer.rs`.
   (Perf work in §6 is the exception — it *must* touch `field.rs`/`struct_.rs`.)
3. **Round-trip is the gold test.** Every writer slice proves itself by writing a
   packet and parsing it back through the generated reader to equality. Add a
   `writer_runtime.rs` case per slice; pin real bytes in `writer_protocol_suite.rs`
   when a protocol gets unblocked.
4. **`cargo clippy --all-targets`** clean at the end of each slice. Generated writer
   code may need `#[allow(...)]` like the reader emits (unused_mut, needless min/max).
5. **Codegen smoke:** `cargo run -p binparse-codegen --example test`.
6. **Delegate** each slice to a background subagent (opus model, per
   [[agent-model-preference]]); supervisor verifies via one build/clippy/test run +
   targeted reads. Keep supervisor context lean.

## 2. The architecture seam (read before touching `writer.rs`)

- Reader flow: `lib.rs → struct_.rs → field.rs → type_/mod.rs → type_/*.rs`.
- `GeneratedLen::{Fixed(Len), Dynamic(TokenStream)}` (in `binparse-codegen/src/lib.rs`)
  is the compile-time-offset vs runtime-offset seam. Writers mirror it.
- `writer.rs` today: `enum Layout { Fixed, DynamicTail, DynamicTailHook,
  DynamicTailOpen, Union }`; `FieldKind { Primitive, BitField, ByteArray, StructRef,
  Constant }`; bit-granular offsets. Entry: `pub(crate) fn generate(ast,
  &HashMap<&str,usize>) -> Result<(TokenStream, Option<usize>), Error>`. Per-struct
  surfaces: Mode 1 (`new` + `set_*` + `*_mut`), Mode 3 (`Content` + `write_into` /
  `to_vec`), `encoded_len`/`SIZE`. Derived fields (length/discriminant/constant) are
  auto-written, no setter, not in `Content`.

## 3. THE GATE — generalize the offset model (do this first)

Everything in bucket A is a *leaf* of one foundational change. Today the writer only
supports **dynamic-at-the-tail**: a fixed prefix at compile-time offsets + one
variable region last. The single architectural step is to let **offsets be runtime
values mid-struct** so a field can *follow* a dynamic field.

- **Model:** replace the per-`Layout` ad-hoc offset tracking with a **forward offset
  accumulator** threaded field-by-field. Each field contributes a `GeneratedLen`
  (Fixed → const, Dynamic → a runtime `Len` expression over the `Lens`/`Content`).
  Field N's start offset = running sum of fields `0..N`. This subsumes `Fixed`,
  `DynamicTail`, `DynamicTailOpen` as special cases.
- **HARD RULE (from spec §1 "Write path"):** the bulk-write paths (`write_into` /
  `to_vec` / union variant writers) emit fields in order advancing a single running
  offset — **O(n), compute each length once**. Never lower a writer offset as a
  recursive *backward* chain (that would reintroduce the reader's O(n²) and is the
  reason the writer needs no `@cache`). Mode-1 standalone `*_mut()` accessors compute
  their own prefix sum from the `Lens` (each a load / cheap `width_fn`); that's fine.
- **Deliverable:** a struct with `[fixed][dynamic byte region][fixed trailer]` writes
  and round-trips. This unblocks arp (interleaved/multiple dynamic) immediately and is
  the prerequisite for every slice below.
- **Verification:** new `writer_runtime.rs` cases with a dynamic field followed by
  more fields; reader snapshots still byte-for-byte; clippy clean.

## 4. Bucket A leaves (each builds on §3, any order after the gate)

Sequenced by leverage (protocol unblocked in parens). Each is one delegated slice.

1. **Affine size exprs** `[u8; len-8]` (udp). Auto-invertible: region length =
   `lenfield_value - k`; derived length field writes back `region_len + k`. Smallest
   slice; good warm-up on the new accumulator.
2. **`@len`-regions** — bounded struct-ref / union / greedy where the region length is
   derived from the content's `encoded_len` (tcp options, mqtt body, tls, bgp). The
   length field is derived (no setter), value = inner `encoded_len`. Composes with the
   write-hook for varint lengths (mqtt) — see §5.
3. **Array-of-structs** `[Inner; count]` and greedy `[Inner]` (sctp, dhcp, dns). Count
   field derived from element count; element offsets via the forward accumulator
   (each element's `encoded_len`). Mode 3 `Content` carries a slice of element
   contents.
4. **Conditionals** `if/else` → `Option` in `Content` (ipv4, dhcp). Present-branch
   fields contribute their length only when the condition (a prior derived/!derived
   field) selects them. Mirror the reader's `Conditional` lowering.
5. **Richer unions** — dynamic variants (variant bodies with their own dynamic
   fields), multi-literal discriminants `4|5`, tuple discriminants `(0,0)`, a
   *writable* wildcard variant, `@error` arms. Generalizes the existing `Union` layout
   (currently fixed-variant, Mode-3-only, non-writable `_ => Unknown{}`).
6. **Multi-byte / non-`u8` arrays + `concat`** — `[u16; n]` etc. and `Type::Concat`
   regions. Endian-aware element writes; concat = sequential sub-region writes through
   the accumulator.
7. **Layout attrs** — `@pad`/`@pad_to`/`@align` (emit zero-fill to the alignment;
   `Len::pad_to` already exists), `@skip`, `@until` tails, struct-level `@len`. Mostly
   accumulator advances + zero-fill.

(Coverage map of what each unblocks is in [[writer-subsystem]]; FULL writers exist
today for ethernet/vlan/ipv6 + partial tcp/sctp/dhcp.)

## 5. Write-hook contract rework (needed for content hooks: dns/tls/mqtt)

Settled design in **spec §2 "Write direction" + §5.8** (read it). Summary of what to
build:

- **One required hook fn** = the symmetric dual of the read hook:
  `fn encode(value: V, dst: &mut [u8], ctx: WriteHookContext) -> WriteResult<usize>`
  (value + mut-buffer + ctx → written). `V` may borrow from the caller's `Content`
  (`&'a [u8]`, label slices, lazy views) — threads the content `'a`.
- **`width(value) -> usize` is OPTIONAL** (sizing accelerator): present → exact
  `encoded_len` + Mode-1 random access kept; absent → write-then-measure (`to_vec`
  grows, `write_into` returns actual + `encoded_len` upper-bounds), and fields after
  the hook go sequential-write-only.
- **`WriteHookContext { offset: usize, written: &[u8] }`** added to `binparse/src/lib.rs`
  (mirror of read `HookContext`) — lets compressing encoders (dns names) scan the
  already-written prefix. **Context-carrying model (B) chosen** (user wants the
  powerful version; compression supported).
- Touchpoints: `writer.rs` `parse_write_hook` (~653, make width optional) +
  `DynamicTailHook` (~60, generalize `u64`→`V`); `attr.rs` shares the `path_to_tokens`
  type-grammar extension with the read borrowed-returns work; `hooks.rs` gains
  borrowed-arg encoder companions (dual of `leb128_unsigned` etc.).
- Today's `@write_hook(encode_fn, width_fn)` length path is the special case `V=u64`
  with width present — keep it working unchanged. Missing `@write_hook` where required
  stays the hard `writer::Error::MissingWriteHook`.

## 6. Perf work (parser path — independent of writers)

Fully specified in `docs/cache-and-borrowed-hooks-spec.md`. Two independent levers
that compose to close the DNS gap (58 hook calls → ~2):

- **`@cache(len|value)`** offset/value memoization — `Cell`/`OnceCell` slots on the
  view; auto offset-memo for dynamic fields recommended. Touches `field.rs`,
  `struct_.rs`, `attr.rs`. **Read-path only** — writers stay cache-free (spec §1
  "Write path").
- **Borrowed hook returns** — `attr.rs` `path_to_tokens` grammar extension to accept
  full types (`&'a [u8]`, `NameRef<'a>`); lifetime threads from the view's `'a`. The
  *same* grammar extension serves the write borrowed-args in §5 — land it once.
- Re-bench DNS after landing (commands + baselines in [[rebench-dns-after-cache-fix]]).

## 7. Bucket C — Mode 2 `writer_over` (do last)

`writer_over(&mut buf)` — infer lens from an existing populated buffer (reuse the
reader's offset walk to discover region lengths), edit in place, no resize. Mechanical
once the offset model (§3) is general. Mode 1 + Mode 2 internals already share the
offset engine.

## 8. Suggested session chunking (to stay within context)

The whole roadmap is too large for one session even with delegation (integration +
verification + debug cycles are irreducibly the supervisor's). Suggested boundaries —
each ends green + clippy-clean + committed:

- **Session A:** §3 gate + §4.1 (affine) + §4.2 (`@len`-regions). The riskiest,
  highest-leverage block; de-risks everything downstream.
- **Session B:** §4.3 (arrays-of-structs) + §4.4 (conditionals) + §4.5 (unions).
- **Session C:** §5 write-hook rework + §6 borrowed-hook-returns (shared grammar) →
  unblocks dns/tls/mqtt content writing.
- **Session D:** §6 `@cache` + DNS re-bench; §4.6/§4.7 leftovers; §7 Mode 2.

Start each session by reading this doc + the spec + MEMORY.md; nothing else needs
re-providing.
