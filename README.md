<div align="center">

# binparse

**Describe a binary format once. Get a zero-copy parser, a writer, and a packet dissector — for free.**

[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![rust](https://img.shields.io/badge/rust-edition%202024-orange.svg?logo=rust)](https://www.rust-lang.org)
[![zero-copy](https://img.shields.io/badge/parsing-zero--copy-success.svg)](#why-binparse)
[![status](https://img.shields.io/badge/status-WIP-yellow.svg)](#status)

</div>

---

binparse turns a tiny declarative spec into hand-written-quality Rust. You write the
wire format in a `.bp` file; codegen gives you back parsers that never copy, never
allocate, and never panic on bad input — plus matching writers and a generic field
tree for tooling.

A parsed value is just a typed view over `&[u8]`. Accessors compute their offsets on
demand, so you only pay for the fields you touch.

## Why binparse

- **Zero-copy by construction.** A parse result borrows the input — getters read
  straight out of the original bytes. No intermediate structs, no heap.
- **Pay-per-field.** Reach for `packet.ttl()` and that's the only offset walk you
  pay for. Ignore the rest and it costs nothing.
- **Never panics.** Feeding a parser arbitrary garbage returns an error or a partial
  field tree — that's a project invariant, fuzzed continuously.
- **Round-trips.** The same spec generates writers, so you can edit a packet in place
  or build one from scratch and serialize it back to bytes.
- **Built for dissectors.** Every field reports its exact bit range, and every struct
  gets a `dissect()` that yields a generic tree — the shape a Wireshark-style UI wants.

## A taste

```
struct Ethernet {
    dst: [u8; 6],
    src: [u8; 6],
    @discriminator ethertype: u16,
    @greedy(unsafe_eof) @payload payload: [u8],
}
```

That single spec generates an `Ethernet<'a>` view with typed accessors and a
`parse(&[u8]) -> Result<(Self, &[u8])>`:

```rust
let (eth, rest) = Ethernet::parse(&frame)?;

eth.ethertype();          // 0x0806
eth.payload()?;           // an iterator over the tail — nothing copied
```

## What the DSL can express

Far more than fixed structs:

| Feature | Looks like |
|---|---|
| Bitfields (sub-byte) | `version: b<4>` |
| Length-prefixed arrays | `payload: [u8; length - 8]` |
| Discriminated unions | `body: union(packet_type) { 1 => Connect { .. }, _ => Unknown {} }` |
| Conditionals | `if has_options { options: [u8; n] }` |
| Hooks for the gnarly bits | `@hook(hooks.dns_name, ..) name: [u8]` (compression, varints, …) |
| Layout control | `@cache`, `@len`, `@pad`, `@align`, `@payload` |

If the parser accepts a spec, codegen either fully supports it or rejects it with a
precise diagnostic. No silent surprises.

## Batteries included: `binparse-protocols`

Fifteen real protocols, generated at build time, each behind its own Cargo feature:

> Ethernet · VLAN · ARP · IPv4/IPv6 · ICMP/ICMPv6 · UDP · TCP · DNS · TLS · DHCP · SCTP · BGP · MQTT v3.1.1 & v5

```toml
binparse-protocols = { features = ["dns", "tcp"] }
```

Nothing is enabled by default — turn on what you need, or `all`.

## Fast where it counts

Zero-copy isn't a slogan here. Benchmarked (criterion) against the best crates in
each niche:

- **DNS** — beats `simple-dns` on both read and write, thanks to cached lengths and
  borrowed name views (~1.9× on the write path).
- **MQTT** — wins outright; decoding-into-owned can't keep up with reading in place
  (~2.5–3× faster than `mqttbytes`/`rumqttc`/`rumqttd` on writes).

`binparse-bench` runs the head-to-heads against `etherparse`, `pnet_packet`,
`tls-parser`, `simple-dns`, `mqttbytes`, `rumqttc`, and `rumqttd`.

## The workspace

| Crate | What it does |
|---|---|
| `binparse-dsl` | The AST — the language definition. |
| `binparse-dsl-parse` | Text `.bp` → AST. |
| `binparse-codegen` | AST → Rust. The heart of the project. |
| `binparse` | The runtime: offsets, errors, hooks, field tree. |
| `binparse-protocols` | The 15 shipped protocol parsers. |
| `binparse-bench` | Criterion benchmarks vs. the field. |
| `binparse-lsp` | Language server: parse + codegen diagnostics. |

## Hacking

```bash
cargo run -p binparse-codegen --example test   # dump the generated Rust
cargo test
cargo clippy --all-targets
```

## Status

Work in progress, not yet on crates.io. The parse path is complete and fast. Writers
ship for the full protocol suite — MQTT v3/v5 and DNS (with codegen-derived name
compression) round-trip and benchmark ahead of the hand-written codecs. The
remaining writer frontier is content-range checksums and a typestate builder. The
dissection API is landing feature by feature.

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
