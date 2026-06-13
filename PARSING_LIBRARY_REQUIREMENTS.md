# Binparse Parsing Library Requirements

This document scopes `binparse` as a parsing library that can be used behind a Wireshark-like application.

The library is not responsible for packet capture, UI, columns, filtering, persistence, stream reassembly, protocol preferences, or expert workflows. A separate crate may build those features on top of the parsing and dissection APIs described here.

## Success Criteria

The library is ready for a dissector-style application when:

- DSL accepted by the parser is either fully supported by codegen or rejected with a precise diagnostic.
- Any byte slice can be parsed without panics.
- Parsers can produce both typed Rust accessors and a generic field tree.
- The field tree contains enough offset, value, and error information for an external UI to render packet details and bytes.
- Protocol specs for Ethernet, VLAN, IPv4, IPv6, UDP, TCP, DNS, and TLS records can be written naturally without Rust code except for explicitly declared hooks.

## Planning Rule

Build the parser first and stabilize public boundaries late.

Requirements should be implemented in small steps that prove real protocol parsing behavior before committing to broad public APIs. Runtime types may start narrow and internal. Public API polish, crate-level documentation, registry traits, and long-term field tree stability belong in publish readiness after the parser can handle representative protocols.

Do not add a general unsupported-feature error layer just to retire `todo!()` in unpublished code. A requirement is complete when the matching codegen/runtime paths are implemented and tested. `todo!()` paths for unrelated future features may stay until those features are implemented. Before publish, user-reachable `todo!()` and panic paths must be gone.

No-panic parsing is a project invariant, not a standalone milestone. It must be kept in mind throughout implementation and verified for each feature as that feature becomes supported.

Every requirement has the same acceptance criteria:

- Generated Rust for that feature compiles.
- Valid packet examples decode to the expected values.
- Malformed or truncated byte slices covering that feature return errors or partial dissection results instead of panicking.
- Existing behavior touched by the change keeps passing.

## R1. Baseline Existing Supported Surface

Goal: make the behavior that already exists in codegen concrete, compiled, and no-panic before adding more features.

Requirements:

- Add generated-code tests for the current intended surface: primitives, endian attributes, bitfields, fixed arrays, expression-sized arrays, fixed-offset struct refs, concat, basic unions, fixed hooks, and variable-length hooks.
- Ensure generated parsers for that surface compile as Rust and are exercised at runtime, not only compared as token strings.
- Enforce the no-panic invariant for `parse()` on those generated specs.
- Enforce the no-panic invariant for field access after a successful `parse()` on those generated specs.
- Ensure nested struct refs and array iterators propagate parse errors instead of unwrapping parse results.
- Leave unrelated future feature `todo!()` paths alone until their requirement is implemented.

Done when:

- Golden tests cover the current intended surface.
- Malformed and truncated packet tests cover the current intended surface.
- A fuzz or property-style test can call generated top-level parsers for the current intended surface with arbitrary bytes without panicking.

## R2. Offset Model

Goal: every parsed value can be mapped back to packet bytes.

Requirements:

- Runtime supports absolute bit offsets, byte offsets, and bit lengths.
- Generated code can compute start and end offsets for fixed and dynamic fields.
- Offset computation works across bitfields, arrays, conditionals, unions, padding, and nested structs.
- Offsets are represented in one common runtime type instead of ad hoc `usize` byte offsets.
- Offset arithmetic checks overflow.

Done when:

- Every field in a parsed struct can report an absolute bit range.
- Tests cover byte-aligned fields, sub-byte fields, cross-byte bitfields, nested structs, and dynamic arrays.

## R3. Expression Semantics

Goal: all DSL expression users share one implementation.

Requirements:

- Implement typed expression lowering for numeric expressions.
- Implement typed expression lowering for boolean expressions.
- Implement field path resolution for previously parsed fields.
- Implement tuple expressions for union discriminants.
- Reject paths that refer to fields not yet available.
- Reject numeric expressions used where a boolean is required, and boolean expressions used where a number is required.
- Reject operations that can overflow unless explicitly marked wrapping.

Done when:

- Array sizes, constraints, validations, conditionals, and union dispatch use the same expression machinery.
- Expression errors point to the expression and the unavailable or mistyped path.

## R4. Primitive And Bitfield Coverage

Goal: common network scalar encodings are first-class.

Requirements:

- Support unsigned integers already present: `u8`, `u16`, `u32`, `u64`, `u128`.
- Add signed integers: `i8`, `i16`, `i32`, `i64`, `i128`.
- Define bitfield order explicitly.
- Implement `@bit_order(msb)` and `@bit_order(lsb)`.
- Set the default bit order deliberately and document it.
- Support endian inheritance at struct and field level for multi-byte integer types.
- Reject endian attributes where they have no meaning.

Done when:

- IPv4 `version` and `ihl` decode correctly using the documented bit order.
- TCP flags can be described without manual Rust hooks.

## R5. Constraints And Validation

Goal: protocol invariants are part of the parser.

Requirements:

- Implement constant fields such as `magic = x89504e47`.
- Implement `@check(expr)` or `@validate(expr)` with boolean expressions.
- Implement `@range(min, max)` for numeric fields.
- Validation failures produce `ParseError::ValidationFailed` with field path and actual value.
- Validation is available to both typed parse and generic dissection.

Done when:

- Magic numbers, version checks, length checks, and reserved-bit checks are expressed in DSL and tested.

## R6. Conditional Fields

Goal: optional protocol fields are expressible without hooks.

Requirements:

- Implement `if (expr) { ... }`.
- Implement `else { ... }`.
- Conditional branches update offsets correctly.
- Fields inside a conditional have stable paths in the dissection tree.
- Later expressions can only reference conditional fields through an explicit optional access mechanism, or such references are rejected.

Done when:

- IPv4 options can be conditionally parsed based on `ihl`.
- TCP options can be conditionally parsed based on `data_offset`.

## R7. Arrays And Variable-Length Data

Goal: length-prefixed and remaining-byte payloads are first-class.

Requirements:

- Implement fixed-size arrays.
- Implement expression-sized arrays.
- Implement consume-rest arrays with explicit syntax or attribute.
- Implement sentinel-terminated arrays, for example `@until(x00)`.
- Implement bounded greedy parsing, for example `@greedy(...)` with `@max_iter(N)`.
- Array sizes must be checked against remaining packet length before iteration can panic.
- Arrays of primitives, bitfields, and struct refs must share consistent error behavior.

Done when:

- DNS label sequences, TLS record bodies, IPv4 options, and TCP options can be represented without custom Rust except where compression or protocol-specific transforms require hooks.

## R8. Padding And Alignment

Goal: byte layout control is explicit.

Requirements:

- Implement `@skip` for fields that consume bytes or bits but are omitted from typed accessors.
- Implement `@align(N)` to require or move to an aligned offset according to documented semantics.
- Implement `@pad(N)` for fixed padding.
- Implement `@pad_to(N)` for padding until an offset boundary.
- Padding appears in the dissection tree as hidden or generated fields according to metadata.

Done when:

- Specs can model reserved bits, padding bytes, and alignment boundaries without naming fake protocol fields.

## R9. Unions And Dispatch

Goal: protocol variants and payload dispatch are expressible in DSL.

Requirements:

- Support single-field union dispatch.
- Support tuple union dispatch.
- Support wildcard matchers.
- Support multiple matchers for one variant.
- Implement error variants declared with `@error(...)`.
- Detect non-exhaustive unions unless a wildcard or explicit error variant exists.
- Union variant lengths may be fixed or dynamic.
- Union-generated enums expose a fallible parse/dissect path for their selected variant.

Done when:

- ICMP type/code bodies and Ethernet EtherType payload dispatch can be represented.

## R10. Length-Limited Nested Parsing

Goal: nested protocol parsers cannot read outside their declared slice.

Requirements:

- Implement field-level length bounding such as `@len(expr) inner: Inner`.
- Implement struct-level length bounding where appropriate.
- Nested parsers receive only the bounded slice.
- Length mismatches can be either validation errors or trailing-byte reports according to explicit policy.
- Payload fields can expose remaining bytes for a higher-level dispatcher.

Done when:

- TLV values, TLS records, DNS RDATA, and length-prefixed application payloads are bounded correctly.

## R11. Hook Interface

Goal: hooks cover genuinely protocol-specific parsing without weakening safety.

Requirements:

- Hooks are fallible.
- Hooks receive a bounded input slice and field context.
- Hooks return bytes consumed.
- Hooks cannot make the generated parser lose offset tracking.
- Hooks can return either typed values, field tree nodes, or both according to trait design.
- Built-in hooks cover cstring, unsigned LEB128, signed LEB128, and common varint patterns.

Done when:

- A hook can parse a DNS compressed name while preserving field offsets and errors.

## R12. Generic Dissection Tree

Goal: external applications can inspect packets without knowing generated Rust types.

Requirements:

- Runtime defines a `FieldNode` or equivalent tree type.
- Each node contains field name, display name, path, type name, absolute bit range, raw byte range when byte-aligned, decoded value, children, and status.
- Decoded values support at least unsigned int, signed int, bool, bytes, string, enum label, struct, array, union variant, absent, and opaque.
- Nodes can represent malformed fields and continue where recovery is possible.
- Generated structs expose a method to append or return their field tree.
- The tree carries enough information for a UI crate to render packet details and byte highlighting.
- The exact public stability of the tree shape is deferred to publish readiness.

Done when:

- A generated parser can produce a tree for Ethernet + IPv4 + UDP + DNS without the UI crate depending on concrete generated protocol types.

## R13. Protocol Handoff Metadata

Goal: a dependent crate can chain parsers without hardcoding every field layout.

Requirements:

- DSL can mark a field as a protocol discriminator.
- DSL can mark a field as payload bytes or payload struct.
- Generated dissection output can expose a handoff key such as EtherType, IP protocol number, UDP port, or TCP port.
- Generated dissection output can expose the payload range associated with the handoff.
- A caller can use the handoff metadata without depending on generated concrete protocol types.

Done when:

- A caller can parse Ethernet, receive an EtherType handoff plus payload range, and choose an IPv4 parser outside this crate.

## R14. Error Recovery

Goal: dissection remains useful on malformed packets.

Requirements:

- Parsing supports fatal errors for impossible continuation.
- Dissection supports recoverable field errors where offsets remain known.
- Errors include field path, offset, expected condition, actual value or length, and source span when caused by generated DSL semantics.
- Generic dissection can return a partial tree plus errors.

Done when:

- Truncated, bad-magic, bad-length, and unknown-variant packets produce partial trees without panics.

## R15. Generated-Code Verification

Goal: generated parsers are tested as compiled Rust, not only as token strings.

This requirement should grow alongside the earlier requirements. Do not wait until all parsing features are implemented before adding generated-code tests.

Requirements:

- Re-enable or replace disabled codegen unit tests.
- Add integration tests that generate Rust from DSL, compile it, and exercise the generated API.
- Add golden packet tests for representative protocols.
- Add malformed packet tests for each supported field kind.
- Add fuzz targets for parser entry points.

Done when:

- `cargo test --all-targets` validates generated parser behavior.
- `cargo clippy --all-targets` is required before merge.
- Fuzzing has at least one target that runs generated parsers over arbitrary input.

## R16. Minimum Protocol Suite

Goal: prove the library is useful for common network protocol descriptions.

Requirements:

- Ethernet II.
- 802.1Q VLAN.
- ARP.
- IPv4 base header.
- IPv4 options.
- IPv6 base header.
- UDP.
- TCP base header.
- TCP options.
- ICMPv4.
- DNS messages enough to parse headers, questions, common RR headers, labels, and compressed names using hooks if needed.
- TLS record layer.

Done when:

- Each protocol has a DSL spec, golden valid packets, malformed packet tests, and generated dissection tree snapshots.

## R17. Publish Readiness And Public Boundaries

Goal: stabilize the library boundary after the parser is useful enough to publish.

Requirements:

- `binparse` provides documented runtime types needed by generated parsers and downstream applications.
- `binparse-dsl` provides only the AST.
- `binparse-dsl-parse` parses source into AST and diagnostics.
- `binparse-codegen` converts AST into Rust parser code.
- Generated code depends on `binparse`, not on UI/application crates.
- No public API introduces UI rendering, packet capture, display filters, persistence, protocol preferences, or session tracking into this crate.
- Public runtime types have documented semantics.
- Generated code uses stable runtime APIs.
- Codegen output is deterministic.
- Errors are non-exhaustive where future variants are expected.
- Field tree value types are versioned or designed for additive extension.
- If a registry trait is needed, it is an interface only; concrete protocol registration lives outside this crate.

Done when:

- A separate application crate can depend on `binparse` and generated protocol parsers without using private codegen internals.
- Public crate responsibilities are documented.
- Application-facing APIs do not depend on generated concrete protocol types.

## Recommended Implementation Order

1. R1: Baseline Existing Supported Surface.
2. R2: Offset Model.
3. R3: Expression Semantics.
4. R4: Primitive And Bitfield Coverage.
5. R5: Constraints And Validation.
6. R6: Conditional Fields.
7. R7: Arrays And Variable-Length Data.
8. R8: Padding And Alignment.
9. R9: Unions And Dispatch.
10. R10: Length-Limited Nested Parsing.
11. R11: Hook Interface.
12. R12: Generic Dissection Tree.
13. R13: Protocol Handoff Metadata.
14. R14: Error Recovery.
15. R16: Minimum Protocol Suite.
16. R17: Publish Readiness And Public Boundaries.

R15 runs throughout the sequence: each implemented requirement should add or update generated-code tests for the behavior it introduces.

## Explicitly Out Of Scope

- Packet capture.
- UI rendering.
- Hex view rendering.
- Display filters.
- Packet list columns.
- Persistence and capture-file formats.
- TCP stream reassembly.
- Conversation tracking.
- Protocol preference UI.
- Plugin loading for external applications.
- Serialization or packet writing.
- Cross-language code generation.
