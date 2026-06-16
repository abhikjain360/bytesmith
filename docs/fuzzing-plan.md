# Fuzzing — Refined Plan & Implementation Guide

A staged plan to take fuzzing from "two synthetic, panic-only-ish targets" to a real
harness that fuzzes the **compiler** (DSL parse + codegen), the **product**
(`bytesmith-protocols` with real hooks), and adds **semantic oracles** (field-tree /
handoff invariants, writer round-trips, narrow differential checks).

This doc is written so a coding agent can pick up any single phase and execute it
without re-deriving the codebase. **Each phase is a self-contained task** with: files
to create, exact APIs (verified against the tree), the oracle (what counts as a bug),
acceptance criteria, and out-of-scope notes.

> **Provenance.** An earlier version of this plan was written against an *older
> commit* and several of its API references had drifted. This version is verified
> against current `main` (tip ~`30749a9`, 2026-06-15). §2 is the corrected API ground
> truth — **use it, not your memory of the old plan.** Items the old plan got wrong
> are flagged with ⚠️.

---

## 0. TL;DR — priority order

Re-ordered from the original by **signal-to-noise** and **reuse of existing assets**:

| # | Task | Why first | Effort | Phase |
|---|------|-----------|--------|-------|
| 1 | Seed corpus + dicts + `fuzz/README.md` | Makes every later target effective; pure data | low | §4.0 |
| 2 | `dsl_parser` target | Parser has exactly **1** known panic site (`lib.rs:205`); clean high-value oracle | low | §4.1 |
| 3 | `dsl_codegen` target | Probes 31 `unreachable!()` + 11 unwrap/expect in codegen | low | §4.1 |
| 4 | Add `bytesmith-protocols` dep + `protocols_raw` | Closes the biggest gap: exercises **real hooks** (DNS) the synthetic target never touches | med | §4.2 |
| 5 | Field-tree + handoff invariant helpers | Turns `protocols_raw`/`protocol_chain` from panic-only into semantic | med | §4.3 |
| 6 | `protocol_chain` (handoff dispatch) | Recursive layer chaining via `handoff()` | med | §4.4 |
| 7 | Strengthen `generated_writers` (negative + modes 1/2) | Cheap; covers `NotEnoughSpace` + the under-fuzzed builder/in-place modes | low | §4.5 |
| 8 | Differential targets (IPv4/UDP/TCP, DNS, MQTT, TLS) | Highest semantic value, highest false-positive risk → narrow rules, do last | high | §4.6 |
| 9 | CI smoke + nightly coverage | Operationalize | med | §4.7 |
| 10 | `Arbitrary` for content structs / ASTs | Only after structured corpora prove their worth | high | §4.8 |

**Keep the existing `generated_parsers` and `generated_writers` targets** as
regression coverage. Do not delete them.

---

## 1. Current state (verified)

### Layout
```
fuzz/
  Cargo.toml          # own workspace (empty [workspace]); own Cargo.lock
  build.rs            # inlines two DSL strings, codegens into $OUT_DIR
  fuzz_targets/
    generated_parsers.rs
    generated_writers.rs
  .gitignore          # ignores: target, corpus, artifacts, coverage
```

- **`fuzz/Cargo.toml`** deps: `libfuzzer-sys = "0.4"`, `bytesmith = { path = ".." }`.
  Build-deps: `bytesmith-codegen`, `bytesmith-dsl-parse`. ⚠️ **No
  `bytesmith-protocols` dependency yet** (the old plan assumed otherwise in places).
  `arbitrary` is present only transitively via `libfuzzer-sys` (unused directly).
- **`fuzz/build.rs`** holds `BASELINE_DSL` and `WRITER_BASELINE_DSL` as inline
  `const &str`, parses each via `bytesmith_dsl_parse::parse_str`, codegens via
  `CodeGen::generate` / `CodeGen::generate_writers`, writes `$OUT_DIR/generated.rs`
  and `$OUT_DIR/generated_writers.rs`. Each target `include!`s its file.
- **`generated_parsers`** — oracle is **panic-only**: `dissect()` then `parse()` on
  ~21 synthetic types, walks getters, no `assert!`. Contains a **local, hand-written
  `parse_dns_name`** hook — it does **not** exercise the shipping
  `bytesmith_protocols::hooks::dns_name`. (Closing that gap is the point of
  `protocols_raw`.)
- **`generated_writers`** — oracle is **round-trip**: a hand-rolled `Cursor` decodes
  fuzz bytes into `*Content` structs, encodes via `to_vec`/`write_into`, asserts
  equality and parses back. Only mode-3 (`to_vec`/`write_into`) is exercised; modes 1
  (`new`+setters) and 2 (`writer_over`) are **not** fuzzed.
- **No `corpus/`, no `dict/`, no `README.md`.** No seed inputs are checked in.

### Existing assets to REUSE (do not reinvent)
- **Bench fixtures** — `bytesmith-bench/src/lib.rs`, 10 `pub const &[u8]`:
  `ETH_FRAME`, `IPV4_PACKET`, `IPV6_PACKET`, `UDP_PACKET`, `TCP_PACKET`,
  `MQTT_V3_CONNECT`, `MQTT_V3_PUBLISH`, `MQTT_V5_CONNACK`, `TLS_RECORD`,
  `DNS_RESPONSE`. (No standalone ARP fixture — ARP appears only as the ethertype
  inside `ETH_FRAME`.) → seed the packet corpus.
- **Wireshark pcap corpus + soak test** — `bytesmith-codegen/tests/pcap_dogfood.rs`
  ⚠️ (lives under `bytesmith-codegen/tests/`, *not* `bytesmith-bench`). It generates a
  throwaway crate from an embedded DSL and soaks `third_party/wireshark/test/captures/`
  (a git submodule: 66 `*.pcap` + 42 `*.pcapng`), asserting no panic on every
  truncation prefix and over the whole pcap corpus. → mine the `.pcap` files (skip the
  pcapng-magic ones) for `protocols_raw` seeds, and keep this test running in CI.
- **Reference codecs** — `bytesmith-bench/Cargo.toml` `[dev-dependencies]`:
  `etherparse 0.20`, `pnet_packet 0.35`, `mqttbytes 0.6`, ⚠️ `rumqttc-v4-next 0.33`
  and `rumqttc-v5-next 0.33` (**not** plain `rumqttc`), `tls-parser 0.12`,
  `simple-dns 0.11`, `rumqttd 0.20`. → the differential targets (§4.6) add these to
  `fuzz/Cargo.toml`.
- **Specs** — `bytesmith-protocols/specs/*.bsm`, **15** files → seed the DSL corpus.

---

## 2. API ground truth ⚠️ (use this, not the old plan)

All entry points are **inherent associated functions on the generated type**, not
trait methods (except where noted). `'a` is the input-buffer lifetime.

### Read / dissect
```rust
// inherent, generated per struct (struct_.rs:354 / :306):
Type::parse(data: &'a [u8]) -> Result<(Type<'a>, &'a [u8]), bytesmith::ParseError>;
Type::dissect(data: &'a [u8]) -> bytesmith::FieldNode<'a>;   // never returns Result

// the Dissect trait (bytesmith/src/lib.rs:27) — &mut self on BOTH:
trait Dissect<'a> {
    fn field_tree(&mut self) -> FieldNode<'a>;
    fn handoff(&mut self) -> Option<Handoff<'a>>;
}
```
- **Field getters take `&mut self`** → bind the parsed view as `mut`. Scalar getters
  return the value (`u8`/`u16`/…). Byte/array/hook fields return
  `ParseResult<Iterator>`; collect with `.collect::<bytesmith::ParseResult<Vec<_>>>()`.
  Union bodies: `view.body().unwrap()` → `<Struct>_body` enum.
- `dissect(data)` does **not** need a binding and never returns `Result`.

### `Handoff` (bytesmith/src/lib.rs:15)
```rust
pub struct Handoff<'a> {
    pub keys: Vec<u128>,            // each @discriminator value widened to u128, decl order
    pub payload: &'a [u8],          // the @payload field bytes
    pub payload_byte_range: Range<usize>,
}
```
Codegen **clamps `start`/`end` into `data.len()`** and returns **`None`** if the
payload offsets are not byte-aligned (struct_.rs:319-332). So
`payload == &data[payload_byte_range]` holds **by construction** and ranges are always
in-bounds. `handoff()` returns `None` for any struct without an `@payload` field.

### Field tree (bytesmith/src/tree.rs:10)
```rust
pub struct FieldNode<'a> {
    pub name: String,
    pub display_name: String,
    pub path: String,
    pub type_name: String,
    pub bit_range: Range<usize>,          // ABSOLUTE within the root packet
    pub byte_range: Option<Range<usize>>, // Some IFF bit_range is byte-aligned
    pub value: Value<'a>,
    pub status: Status,
    pub hidden: bool,
    pub children: Vec<FieldNode<'a>>,
}
impl FieldNode<'a> {
    pub fn errors(&self) -> Vec<(&str, &Status)>; // pre-order, every non-Ok node (path, status)
    pub fn set_paths(&mut self, prefix: &str);    // codegen calls set_paths("") on the root
    pub fn render(&self) -> String;
}
pub enum Status { Ok, Error(ParseError), Failed(&'static str) }   // #[non_exhaustive]
pub enum Value<'a> { UInt(u128), Int(i128), Bool(bool), Bytes(&'a [u8]), String(String),
                     EnumLabel(&'static str), Struct, Array, UnionVariant(&'static str),
                     Absent, Opaque }                              // #[non_exhaustive]
```
⚠️ vs old plan: there are **both** `name` and `display_name`, plus `type_name` and
`hidden`. `byte_range` is `Some` **only** when byte-aligned. `bit_range` is absolute.

### Errors
```rust
pub enum ParseError {  // bytesmith/src/lib.rs:82, #[non_exhaustive]
    NotEnoughData { expected: usize, got: usize },
    UnalignedLength(Len),
    ValidationFailed { field: &'static str, actual: u128 },
    MaxIterationsExceeded { field: &'static str, max: usize },
    Misaligned { field: &'static str, align: usize, offset: Len },
    HookFailed { field: &'static str, reason: &'static str },
}
pub enum WriteError {  // bytesmith/src/lib.rs:122, #[non_exhaustive]
    NotEnoughSpace { expected: usize, got: usize },
    ValueTooLarge { field: &'static str, value: usize, max: usize },
    InvalidContent,
}
```

### Writer API ⚠️ (there is **no `Writer` trait**)
Per writable `Foo`, codegen emits `FooWriter`, `FooContent`, and — **for
variable-length types only** — `FooLens`. Union body content is a `<Struct>BodyContent`
enum (e.g. `MqttPacketBodyContent`, `TcpOptionBodyContent`), distinct from the parsed
`<Struct>_body`. Three usage modes (all verified in
`bytesmith-protocols/tests/writers.rs`):

```rust
// Mode 3 — content struct → bytes (the only mode currently fuzzed):
FooWriter::to_vec(content: &FooContent) -> Vec<u8>;
FooWriter::write_into(buf: &mut [u8], content: &FooContent) -> bytesmith::WriteResult<usize>;

// length (⚠️ signature differs by shape):
FooWriter::encoded_len(content: &FooContent) -> usize;  // FIXED-size types
FooWriter::encoded_len(lens: &FooLens) -> usize;        // VARIABLE-length types

// fixed-size types also expose:
FooWriter::SIZE: usize;                                  // const

// Mode 1 — preallocate + setters (variable-length: takes Lens):
let mut w = FooWriter::new(&mut buf, lens)?;             // Err(NotEnoughSpace) if buf small
w.set_scalar(v); w.bytes_field_mut().copy_from_slice(..);

// Mode 2 — edit fixed fields in place over an existing valid packet:
let mut w = FooWriter::writer_over(&mut buf)?;
w.set_scalar(v);
```
Derived/constant fields (e.g. VLAN `tpid`, UDP `length`, IPv6 `payload_len`) have **no
setter** — the writer computes them. `encoded_len(content|lens)`-vs-shape is the most
common compile error when writing writer fuzz code; check the generated module.

### Protocol modules & top-level types ⚠️
Module name = feature = spec stem. Access as `bytesmith_protocols::<module>::<Type>`.

| module | top-level parser type(s) |
|--------|--------------------------|
| `ethernet` | `EthernetII` (⚠️ not `Ethernet`) |
| `arp` | `Arp` |
| `vlan` | `Vlan` |
| `ip` | `Ipv4`, `Ipv6` (⚠️ one `ip` module/feature, **no** `ipv4`/`ipv6`) |
| `icmp` | `Icmpv4` (⚠️ not `Icmp`) |
| `icmpv6` | `Icmpv6` |
| `udp` | `Udp` |
| `tcp` | `Tcp` (also `TcpOption`, `TcpOptionList`) |
| `dns` | `Dns` |
| `tls` | `TlsRecord`, `TlsStream` (⚠️ not `Tls`) |
| `dhcp` | `Dhcp` (also `DhcpOption`) |
| `sctp` | `Sctp` (also `SctpChunk`) |
| `bgp` | `Bgp` |
| `mqtt_v3` | `MqttPacket` ⚠️ |
| `mqtt_v5` | `MqttPacket` ⚠️ (**same name** as v3 — disambiguate by module path) |

Features: `all` enables all 15 (via the `mqtt` aggregate → `mqtt_v3` + `mqtt_v5`).
Build with `--features all`. Each module is generated at build time into
`$OUT_DIR/protocols.rs`.

### Hooks ⚠️
- **DNS** (`bytesmith_protocols::hooks`): `dns_name(&[u8], HookContext) -> ParseResult<(NameRef<'a>, usize)>`,
  `write_dns_name(..)`, types `NameRef<'a>` (`.labels() -> DnsLabelIter`), `DnsLabelIter`.
  Compression-pointer walk capped at **8 jumps** → `HookFailed { reason: "too many DNS
  compression jumps" }`. Exercised by `Dns::parse` then `dns.qname()?.labels()`.
- **LEB128 / varint** (`bytesmith::hooks`, ⚠️ the *runtime* crate, not protocols):
  `leb128_unsigned`, `leb128_signed`, `zigzag_varint`, `quic_varint`,
  `length_prefixed_bytes`, `backref_blob`, plus `cstring`, and write/len helpers
  (`write_leb128_unsigned`, `leb128_unsigned_len`, …). MQTT specs reference these.

### DSL parse / codegen API
```rust
bytesmith_dsl_parse::parse_str(src: &str)          -> Result<Vec<ast::Definition<'_>>, String>;
bytesmith_dsl_parse::parse_str_located(src: &str)  -> Result<Vec<ast::Definition<'_>>, ParseError>;  // {offset, message}
bytesmith_dsl_parse::parse_str_recover(src: &str)  -> (Vec<ast::Definition<'_>>, Vec<ParseError>);   // editor recovery

bytesmith_codegen::CodeGen::generate(ast: &[ast::Definition]) -> Result<String, Error>;
bytesmith_codegen::CodeGen::generate_writers(ast: &[ast::Definition]) -> Result<String, Error>;
```
- ⚠️ The returned `Vec<Definition>` **borrows `src`** — keep `src` alive across the
  `generate` call.
- `generate*` run `syn::parse2::<syn::File>` on the emitted tokens (lib.rs:239) →
  invalid generated Rust comes back as `Err(Error::InvalidGeneratedCode { .. })`,
  **not** a panic.

### Panic surface (what the compiler targets will actually find)
- **`bytesmith-dsl-parse`**: exactly **one** panic site in parser logic —
  `exprs.pop().unwrap()` at `bytesmith-dsl-parse/src/lib.rs:205` (expression parsing).
  All other `panic!`s are in the `#[cfg(test)]` module (line 771+). So `dsl_parser` is
  low-noise: a panic ≈ a real robustness bug.
- **`bytesmith-codegen`**: **31 `unreachable!()` + 11 `panic!`/`unwrap`/`expect`, zero
  `todo!()`** in `src/`. A tripped `unreachable!()` on parser-accepted DSL is a genuine
  bug (a wrong invariant or missing case). So `dsl_codegen`'s no-panic oracle is
  meaningful, **not** swamped by `todo!()` noise.

---

## 3. Ground rules (the "is it a bug?" decision)

**A finding is a BUG when:**
- a target **panics** (incl. `unwrap`/`expect`/`unreachable!`/arithmetic overflow),
- it **hangs** (libFuzzer `-timeout`),
- it **OOMs** beyond `-rss_limit_mb`,
- a declared **invariant assertion** fails (§4.3),
- a **differential** check fails under the *narrow, agreed* rules (§4.6).

**A finding is NOT a bug:**
- any `Err` return — `ParseError`, codegen `Error` (**including
  `InvalidGeneratedCode`**), `WriteError`. The DSL/specs are intentionally partial,
  lazy, and may be looser/tighter than a full reference library.
- a partial parse (non-empty `rest`), or bytesmith accepting/rejecting bytes a
  reference library does not (until §4.6 proves the languages match for a subset).

**Discipline:**
- **Never `catch_unwind`** in a target — let libFuzzer see panics as crashes.
- **Don't assert success/failure agreement** with reference libs initially (§4.6).
- Treat `Err(InvalidGeneratedCode)` from `dsl_codegen` as a *soft* signal worth a
  ticket (latent codegen bug), but it does **not** fail the fuzz run.
- Run `cargo clippy --all-targets` at the end of every step.

---

## 4. Phases

> Build/run requires nightly: `cargo +nightly fuzz build`. The `fuzz/` crate is its
> **own workspace** with its **own `Cargo.lock`** — adding a dependency means editing
> `fuzz/Cargo.toml` and letting its lock update.

### 4.0 — Operational: corpus, dictionaries, README

**Create:**
```
fuzz/README.md
fuzz/dict/bytesmith_dsl.dict
fuzz/dict/network.dict
fuzz/corpus/dsl_parser/
fuzz/corpus/dsl_codegen/
fuzz/corpus/generated_parsers/
fuzz/corpus/generated_writers/
fuzz/corpus/protocols_raw/
fuzz/corpus/protocol_chain/
```
> ⚠️ `fuzz/.gitignore` currently ignores `corpus/`. To **commit** seed corpora, either
> remove that line or add `!fuzz/corpus/**` un-ignore rules. Decide explicitly and note
> it in the README. (Committing seeds is recommended — they double as regression
> inputs.)

**Seed sources (script it; don't hand-copy):**
- `corpus/dsl_parser/` and `corpus/dsl_codegen/`: every `bytesmith-protocols/specs/*.bsm`
  (15 files) + the two inline DSLs from `fuzz/build.rs`.
- `corpus/generated_parsers/`, `corpus/protocols_raw/`, `corpus/protocol_chain/`: the 10
  `&[u8]` consts from `bytesmith-bench/src/lib.rs`, written as raw binary files; plus
  raw `.pcap`-extracted frames from `third_party/wireshark/test/captures/` (skip files
  whose first 4 bytes are the pcapng magic `0a 0d 0d 0a`). Reuse the magic-classify
  logic in `bytesmith-codegen/tests/pcap_dogfood.rs` as a guide. **Do not** make the
  raw targets parse pcap containers — extract frames offline.
- `corpus/generated_writers/`: a handful of small random byte files (the `Cursor`
  driver tolerates any length).

**Dictionaries** (libFuzzer `-dict=`):
- `bytesmith_dsl.dict` — DSL keywords/attributes/tokens. Source of truth for the token
  set is `bytesmith-dsl-parse/src/lib.rs` and the AST in `bytesmith-dsl/src/lib.rs`;
  enumerate the real attribute names rather than guessing. Seed list (verify against
  the parser, prune anything it doesn't accept):
  `"struct" "union" "error" "concat" "if" "else" "u8" "u16" "u32" "u64" "i8" "i16"
  "i32" "i64" "b<1>" "b<4>" "b<7>" "@endian" "@bit_order" "little" "big"
  "@hook" "@write_hook" "@cache" "@greedy" "unsafe_eof" "@until" "@max_iter"
  "@len" "@range" "@validate" "@align" "@pad" "@pad_to" "@skip" "@discriminator"
  "@payload" "@error"`.
- `network.dict` — protocol gate bytes: ethertypes `\x08\x00 \x08\x06 \x86\xdd
  \x81\x00`, IP version nibbles `\x45 \x60`, IP protos `\x01 \x06 \x11 \x3a \x84`,
  ports `\x00\x35` (53) `\x07\x5b` (1883) `\x22\xb8` (8883) `\x01\xbb` (443)
  `\x03\x55` (853), MQTT `MQTT`, DNS compression `\xc0\x0c`, TLS `\x16\x03\x01
  \x16\x03\x03`, DHCP magic `\x63\x82\x53\x63`.

**`fuzz/README.md`** documents: prerequisites (`cargo +nightly fuzz`), per-target run
commands (§4.7), the "is it a bug?" rules (§3), coverage + corpus-minimization
(`cargo +nightly fuzz cmin`, `cov`), and the corpus-gitignore decision.

**Acceptance:** corpus dirs populated by a committed seeding script; `cargo +nightly
fuzz build` still succeeds; README runnable verbatim.

---

### 4.1 — Compiler targets: `dsl_parser`, `dsl_codegen`

Deps already present (`bytesmith-dsl-parse`, `bytesmith-codegen` are build-deps; promote
to `[dependencies]` of the fuzz crate so the targets can call them at runtime).

Add to `fuzz/Cargo.toml`:
```toml
[dependencies]
bytesmith-dsl-parse = { path = "../bytesmith-dsl-parse" }
bytesmith-codegen   = { path = "../bytesmith-codegen" }

[[bin]]
name = "dsl_parser"
path = "fuzz_targets/dsl_parser.rs"
test = false
doc = false
bench = false

[[bin]]
name = "dsl_codegen"
path = "fuzz_targets/dsl_codegen.rs"
test = false
doc = false
bench = false
```

`fuzz_targets/dsl_parser.rs`:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };
    let _ = bytesmith_dsl_parse::parse_str(src);
    let _ = bytesmith_dsl_parse::parse_str_recover(src); // also fuzz the recovery path
});
```

`fuzz_targets/dsl_codegen.rs`:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(src) = std::str::from_utf8(data) else { return };
    let Ok(ast) = bytesmith_dsl_parse::parse_str(src) else { return }; // src kept alive below
    let _ = bytesmith_codegen::CodeGen::generate(&ast);
    let _ = bytesmith_codegen::CodeGen::generate_writers(&ast);
});
```

**Oracle:** no panic / hang / OOM. `Err` is fine. **Do not** compile the generated Rust
in the loop (`generate` already does `syn::parse2`; full type-checking arbitrary DSL is
out of scope — see §5).

**Expectations / caveats:**
- `dsl_parser` may surface `lib.rs:205 exprs.pop().unwrap()`. When it does, the fix is
  to make that path return a `ParseError`, not to suppress the crash.
- `dsl_codegen` will probe the 31 `unreachable!()` + 11 unwrap/expect. Each distinct
  crash → triage: convert to a typed `Error` variant or fix the invariant.
- Optionally collect `Err(InvalidGeneratedCode)` cases to a side file for later review
  (soft codegen bugs) — but they must not fail the run.

**Acceptance:** both targets build and run ≥60s on the seeded corpus with no *spurious*
crashes (real crashes are tickets, not blockers); clippy clean.

---

### 4.2 — Product target: `protocols_raw`

Add to `fuzz/Cargo.toml`:
```toml
[dependencies]
bytesmith-protocols = { path = "../bytesmith-protocols", features = ["all"] }

[[bin]]
name = "protocols_raw"
path = "fuzz_targets/protocols_raw.rs"
test = false
doc = false
bench = false
```

Feed the **same `&[u8]`** into every shipping top-level parser (§2 table) and exercise
the full surface, importing real hooks (no duplicated synthetic hooks):

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use bytesmith::ParseResult;
use bytesmith_protocols as bp;

// helper: drain an iterator-returning getter without keeping the Vec
macro_rules! drain { ($e:expr) => { if let Ok(it) = $e { let _ = it.collect::<ParseResult<Vec<_>>>(); } } }

fuzz_target!(|data: &[u8]| {
    // dissect() never returns Result — build the tree (and check invariants in §4.3)
    let _ = bp::ethernet::EthernetII::dissect(data);
    if let Ok((mut eth, _rest)) = bp::ethernet::EthernetII::parse(data) {
        let _ = eth.ethertype();
        drain!(eth.payload());
        let _ = eth.field_tree();
        let _ = eth.handoff();
    }

    let _ = bp::ip::Ipv4::dissect(data);
    if let Ok((mut ip, _)) = bp::ip::Ipv4::parse(data) {
        let _ = (ip.version(), ip.ihl(), ip.total_len(), ip.ttl(), ip.protocol());
        let _ = ip.field_tree();
        let _ = ip.handoff();
    }
    if let Ok((mut ip, _)) = bp::ip::Ipv6::parse(data) {
        let _ = (ip.version(), ip.next_header(), ip.payload_len());
        let _ = ip.field_tree();
    }

    // DNS exercises the REAL bytesmith_protocols::hooks::dns_name + label iterator:
    let _ = bp::dns::Dns::dissect(data);
    if let Ok((mut dns, _)) = bp::dns::Dns::parse(data) {
        let _ = (dns.id(), dns.qdcount(), dns.ancount());
        if let Ok(name) = dns.qname() { for _l in name.labels() {} }
        let _ = dns.field_tree();
    }

    // ... repeat for: arp::Arp, vlan::Vlan, icmp::Icmpv4, icmpv6::Icmpv6,
    //     udp::Udp, tcp::Tcp (+ TcpOptionList), tls::TlsRecord, tls::TlsStream,
    //     dhcp::Dhcp, sctp::Sctp, bgp::Bgp,
    //     mqtt_v3::MqttPacket, mqtt_v5::MqttPacket (disambiguate by path).
    // For union-body types also: `if let Ok(b) = view.body() { match b { .. } }`.
});
```

**Notes:**
- Getters need a `mut` binding; `dissect` does not.
- Iterator getters on adversarial lengths can allocate → set `-rss_limit_mb`. Array
  iteration is bounded by `@max_iter` (`ParseError::MaxIterationsExceeded`), so it's
  bounded, not infinite.
- Both MQTT modules export `MqttPacket`; use full paths
  (`bp::mqtt_v3::MqttPacket` / `bp::mqtt_v5::MqttPacket`).

**Oracle:** no panic / hang / OOM. (Add the §4.3 invariant checks once that helper
lands.)

**Acceptance:** builds with `--features all`; runs on the packet corpus with no crash;
manually confirm via coverage that `bytesmith_protocols::hooks::dns_name` is reached.

---

### 4.3 — Shared invariant helpers

Create `fuzz/fuzz_targets/common/mod.rs`; include in a target with
`#[path = "common/mod.rs"] mod common;`. Keep it **generic** over `FieldNode` /
`Handoff` (no per-struct knowledge). `FieldNode`/`Handoff` fields are `pub`.

```rust
use bytesmith::{FieldNode, Handoff, Status};

/// Structural invariants for any dissection tree. `input_len` is data.len().
pub fn check_field_tree(node: &FieldNode<'_>, input_len: usize) {
    assert!(node.bit_range.start <= node.bit_range.end, "inverted bit_range at {}", node.path);
    match &node.byte_range {
        Some(br) => {
            assert_eq!(node.bit_range.start % 8, 0);
            assert_eq!(node.bit_range.end % 8, 0);
            assert_eq!(*br, (node.bit_range.start / 8)..(node.bit_range.end / 8));
        }
        None => assert!(
            node.bit_range.start % 8 != 0 || node.bit_range.end % 8 != 0,
            "byte_range None but bit_range is byte-aligned at {}", node.path,
        ),
    }
    // Input-bounds: enforce ONLY for Ok nodes initially (error/partial trees may
    // intentionally record an expected range beyond available data).
    if matches!(node.status, Status::Ok) {
        assert!(node.bit_range.end <= input_len * 8, "Ok node past input at {}", node.path);
    }
    for child in &node.children {
        // paths are set via root.set_paths("") in codegen; confirm exact root/child
        // path semantics against bytesmith/src/tree.rs:114 before tightening this.
        check_field_tree(child, input_len);
    }
}

/// Invariants for a successful parse's handoff. Call only when parse() returned Ok and
/// you can recompute consumed = data.len() - rest.len().
pub fn check_handoff(h: &Handoff<'_>, data: &[u8], consumed: usize) {
    assert!(h.payload_byte_range.start <= h.payload_byte_range.end);
    assert!(h.payload_byte_range.end <= data.len());
    assert!(h.payload_byte_range.end <= consumed, "payload extends past consumed");
    assert_eq!(h.payload, &data[h.payload_byte_range.clone()]);
}
```

**Caveats baked into the helper:**
- The input-bounds invariant is the one the old plan flagged as risky → gated to
  `Status::Ok` nodes only. Loosen/tighten after the first runs tell you the truth.
- `check_handoff`'s in-bounds asserts hold *by construction* today (codegen clamps) —
  keep them anyway as regression guards against future codegen changes.
- The parent/child **path-prefix** invariant is intentionally left as a TODO until the
  exact `set_paths("")` root-path convention is confirmed at `tree.rs:114`. Don't
  assert a prefix rule you haven't verified — a wrong assertion is a false bug.

Wire `check_field_tree(&tree, data.len())` into `protocols_raw` (and the synthetic
`generated_parsers`) on every `dissect()`/`field_tree()` result.

**Acceptance:** helper compiles; wired into ≥1 target; no invariant failures on the
seed corpus (a failure here is a real bug — file it).

---

### 4.4 — `protocol_chain`

Recursive layer dispatch driven by **`handoff()`** — there is no central
ethertype→type registry; you map `handoff().keys` to the next parser yourself and feed
it `handoff().payload`.

```toml
[[bin]]
name = "protocol_chain"
path = "fuzz_targets/protocol_chain.rs"
test = false
doc = false
bench = false
```

Dispatch table (verify each layer actually declares `@payload` + `@discriminator` in
its spec; `handoff()` returns `None` otherwise — handle that, don't unwrap):

```
EthernetII.handoff().keys[0] (ethertype):
  0x8100 -> vlan::Vlan      0x0806 -> arp::Arp
  0x0800 -> ip::Ipv4        0x86dd -> ip::Ipv6
Ipv4.protocol / Ipv6.next_header:
  1 -> icmp::Icmpv4   58 -> icmpv6::Icmpv6   6 -> tcp::Tcp   17 -> udp::Udp   132 -> sctp::Sctp
Udp/Tcp by port (src or dst):
  53 -> dns::Dns     1883|8883 -> mqtt_v{3,5}::MqttPacket     443|853 -> tls::TlsRecord
```

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use bytesmith_protocols as bp;

const MAX_DEPTH: u32 = 8;

fuzz_target!(|data: &[u8]| {
    if let Ok((mut eth, _)) = bp::ethernet::EthernetII::parse(data) {
        if let Some(h) = eth.handoff() {
            dispatch_l3(h.keys.first().copied(), h.payload, 0);
        }
    }
});

fn dispatch_l3(ethertype: Option<u128>, payload: &[u8], depth: u32) {
    if depth >= MAX_DEPTH { return; }
    match ethertype {
        Some(0x0800) => if let Ok((mut ip, _)) = bp::ip::Ipv4::parse(payload) {
            let _ = ip.field_tree();
            if let Some(h) = ip.handoff() { dispatch_l4(h.keys.first().copied(), h.payload, depth + 1); }
        },
        Some(0x86dd) => { /* Ipv6 → dispatch_l4 */ }
        Some(0x8100) => { /* Vlan → recurse dispatch_l3 with inner ethertype */ }
        Some(0x0806) => { let _ = bp::arp::Arp::parse(payload); }
        _ => {}
    }
}
fn dispatch_l4(proto: Option<u128>, payload: &[u8], depth: u32) { /* 6/17/1/58/132 + ports → DNS/MQTT/TLS */ }
```

**Rules:** bound recursion (`MAX_DEPTH`); never recurse on arbitrary payloads forever;
handle `handoff() == None`; run `check_handoff`/`check_field_tree` (§4.3) at each hop.

**Acceptance:** builds; runs bounded; finds no panic; coverage shows multi-layer chains
reached (Eth→IP→UDP→DNS at minimum).

---

### 4.5 — Strengthen `generated_writers`

Keep the existing target; add properties (before any `Arbitrary` rewrite):

1. **Negative buffer** → `NotEnoughSpace`, not panic:
   ```rust
   let need = FooWriter::encoded_len(&lens);          // or &content for fixed types
   if need > 0 {
       let mut small = vec![0u8; need - 1];
       assert!(matches!(FooWriter::write_into(&mut small, &content),
                        Err(bytesmith::WriteError::NotEnoughSpace { .. })));
   }
   ```
2. **Stability:** `assert_eq!(FooWriter::to_vec(&content), FooWriter::to_vec(&content))`.
3. **Round-trip getters:** `parse(to_vec(content))` succeeds and getters equal content
   (already partly done — extend to all writer types).
4. **Modes 1 & 2 (currently un-fuzzed):** for a variable-length type, also drive
   `FooWriter::new(&mut buf, lens)` + setters/`*_mut()`, and `writer_over(&mut buf)` for
   in-place fixed-field edits, asserting the result re-parses. Use a real protocol's
   writer (e.g. `EthernetII`, `Udp`) so this overlaps `protocols_raw` coverage.

**Note:** the synthetic `WRITER_BASELINE_DSL` already covers fixed/var/union/bits/varint
shapes; you can also point a writer fuzz target at real `bytesmith-protocols` writers
once `bytesmith-protocols` is a dep (§4.2). Prefer that over growing the synthetic DSL.

**Acceptance:** new asserts pass on the corpus; `NotEnoughSpace` path proven; clippy
clean.

---

### 4.6 — Differential targets (do last; narrow rules)

Add reference codecs to `fuzz/Cargo.toml` (⚠️ exact names):
```toml
etherparse        = "0.20"
pnet_packet       = "0.35"
simple-dns        = "0.11"
mqttbytes         = "0.6"
rumqttc-v4-next   = "0.33"
rumqttc-v5-next   = "0.33"
tls-parser        = "0.12"
```
Targets: `diff_ipv4_tcp_udp`, `diff_dns`, `diff_mqtt`, `diff_tls_lite`.

**Comparison rule (strict):** only when **both** parsers accept, compare a small set of
**stable scalar fields**; never assert success/failure agreement, never compare
zero-copy iterator contents wholesale. Suggested fields:
- **IPv4:** version, ihl/header_len, total_len, ttl, protocol, src, dst.
- **UDP:** src_port, dst_port, length. **TCP:** src/dst port, seq, ack, data_offset,
  flags.
- **DNS:** id, qd/an/ns/ar count; first qtype/qclass; A/AAAA answer bytes when both
  expose them.
- **MQTT:** packet type, remaining length; selected CONNECT/CONNACK/PUBLISH fields for
  valid packets.

Only promote a protocol to "success/failure agreement" after you've **proved** bytesmith
and the reference accept the same sub-language for that subset.

**Acceptance per target:** builds; runs on the seeded packet corpus; any mismatch
reproduces deterministically and is triaged against the narrow rule (often the rule, not
the parser, needs tightening first).

---

### 4.7 — CI: smoke + nightly

**PR smoke** (fast, corpus-only / short timed):
```bash
cargo +nightly fuzz build
for t in dsl_parser dsl_codegen protocols_raw protocol_chain generated_parsers generated_writers; do
  cargo +nightly fuzz run "$t" fuzz/corpus/"$t" -- \
    -max_total_time=30 -rss_limit_mb=2048 -timeout=10 \
    ${DICT:+-dict=$DICT}   # bytesmith_dsl.dict for dsl_*, network.dict for raw/chain
done
```
Also keep `cargo test -p bytesmith-codegen --test pcap_dogfood` in CI — it already soaks
the wireshark corpus for no-panic.

**Nightly/dev:** long runs (`-max_total_time=3600+`), `cargo +nightly fuzz coverage`,
and `cargo +nightly fuzz cmin` corpus minimization. Track coverage; **don't claim
"proved no-panic"** unless fuzzing runs continuously and coverage is reported.

> ⚠️ Update `TODO.md` only to reflect reality: today it marks crash-resistance +
> round-trip `[x]` but overall "Fuzzing integration" as *partial*. Flip individual
> boxes (`Arbitrary`, `proptest`, real-protocol fuzzing) as they actually land — don't
> mark "done" on the strength of targets existing.

**Acceptance:** CI job green; nightly produces a coverage artifact.

---

### 4.8 — `Arbitrary` (last)

Only after structured corpora + dicts prove insufficient. Add `arbitrary` as a **direct**
dep and derive/hand-write `Arbitrary` for `*Content` structs (and optionally a small AST
generator for `dsl_codegen`). This replaces the hand-rolled `Cursor` in
`generated_writers`. Do not do this first — the immediate wins are negative-buffer +
real-protocol coverage.

---

## 5. Compile-checking generated code (separate from fuzzing)

Do **not** compile arbitrary generated Rust inside libFuzzer. For compile-checking
*curated valid* DSL, reuse the existing throwaway-runtime-crate pattern in
`bytesmith-codegen/tests/pcap_dogfood.rs` (it writes a temp crate under
`target/generated-runtime-tests/` and shells `cargo test`). Add curated cases there or
in `bytesmith-codegen/tests/`, not in a fuzz target.

---

## 6. Codebase gotchas (the things that break a naive target build)

- **fuzz is its own workspace** (`[workspace]` in `fuzz/Cargo.toml`) with its own
  `Cargo.lock`; new deps update that lock, not the root one.
- **`src` lifetime:** `parse_str` returns `Vec<Definition<'_>>` borrowing the source —
  keep the `&str` alive across `generate`.
- **`mut` bindings:** every parsed-view getter and `field_tree`/`handoff` is `&mut self`.
- **`MqttPacket` name collision** across `mqtt_v3`/`mqtt_v5` — always qualify by module.
- **`ip` module** holds both `Ipv4` and `Ipv6`; there is no `ipv4`/`ipv6` feature.
  `EthernetII` (not `Ethernet`), `Icmpv4` (not `Icmp`), `TlsRecord`/`TlsStream` (not
  `Tls`).
- **`encoded_len` signature differs by shape:** `&Content` (fixed) vs `&Lens`
  (variable) — check the generated module.
- **`handoff()` is `None`** for any struct without `@payload`; many leaf protocols won't
  have it.
- **OOM/alloc:** iterator getters and `field_tree` allocate proportional to input; set
  `-rss_limit_mb`. Array growth is bounded by `@max_iter`.
- **DNS hook divergence:** the synthetic `generated_parsers` `parse_dns_name` is a
  *local copy* and does not exercise `bytesmith_protocols::hooks::dns_name`. Only
  `protocols_raw`/`protocol_chain`/`diff_dns` hit the real hook.
- `dissect()` exists and is generated unconditionally (`struct_.rs:306`) — earlier
  "no dissect" claims came from grepping only hand-written source, not codegen output.

---

## 7. Per-phase copy-paste brief (hand one block to an agent)

```
Context: read docs/fuzzing-plan.md §2 (API ground truth) and §3 (bug rules) first.
The fuzz/ crate is its own workspace; run `cargo +nightly fuzz build` to verify, and
`cargo clippy --all-targets` at the end. Keep existing targets intact.

§4.0  Add fuzz/README.md, fuzz/dict/{bytesmith_dsl,network}.dict, corpus dirs, and a
      committed seeding script (specs/*.bsm + build.rs DSLs → dsl corpora; bench consts
      + extracted wireshark .pcap frames → packet corpora). Resolve the corpus
      .gitignore decision in the README.

§4.1  Promote bytesmith-dsl-parse + bytesmith-codegen to [dependencies]; add dsl_parser
      and dsl_codegen targets (see skeletons). Oracle: no panic/hang/OOM; Err is fine;
      do NOT compile generated code. Triage any crash (parser lib.rs:205; codegen
      unreachable!/unwrap) into a typed Err or a real fix.

§4.2  Add bytesmith-protocols { features=["all"] }; add protocols_raw feeding the same
      bytes into every §2 top-level type, calling dissect/parse/field_tree/handoff,
      representative getters, and the real DNS hook via Dns::qname().labels(). Qualify
      MqttPacket by module. Set -rss_limit_mb.

§4.3  Add fuzz_targets/common/mod.rs with check_field_tree + check_handoff (skeleton in
      doc). Gate input-bounds to Status::Ok nodes; leave the path-prefix invariant TODO
      until tree.rs:114 set_paths semantics are confirmed. Wire into protocols_raw.

§4.4  Add protocol_chain: Eth→(VLAN/ARP/IPv4/IPv6)→(ICMP/TCP/UDP/SCTP)→(DNS/MQTT/TLS)
      via handoff().keys + handoff().payload. Bound recursion; handle handoff()==None.

§4.5  Strengthen generated_writers: too-small-buffer => NotEnoughSpace; to_vec
      stability; parse(to_vec)==content; exercise mode-1 (new+setters) and mode-2
      (writer_over) on a real protocol writer.

§4.6  Differential targets (last): add etherparse/pnet_packet/simple-dns/mqttbytes/
      rumqttc-v4-next/rumqttc-v5-next/tls-parser. Compare only stable scalar fields when
      both accept. Never assert success/failure agreement initially.

§4.7  CI smoke loop + nightly coverage/cmin; keep pcap_dogfood in CI; update TODO.md to
      reality.

§4.8  Only then: Arbitrary for *Content structs / AST.
```
