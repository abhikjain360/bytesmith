# Binparse DSL Roadmap

## Why This Over AI-Generated Code?

The goal is to provide **guarantees AI cannot**, **automation AI cannot maintain**, and **consistency across protocol families**.

---

## Core Parsing Features

### Hooks (Rust Integration)

- [x] `@hook(fn_name, ReturnTypePathWithAllRequiredGenerics)` attribute for custom parsing
  - [x] Hook returns `(value in correct type, bytes consumed)` tuple
  - [x] Hook has access to remaining slice
  - [x] Propagate dynamic length to subsequent fields
- [x] Built-in hooks library
  - [x] `cstring` - null-terminated string
  - [x] `leb128` - signed/unsigned LEB128

### Array Improvements

- [x] Arrays with expression-based size: `[u8; header.len - 4]`
- [x] Dynamic start offset support (previously `todo!()` in array.rs)

### Validation

- [x] `@validate(expr)` attribute for field constraints
- [x] `@range(min, max)` for numeric bounds
- [x] Return `ParseError::ValidationFailed` on violation
- [x] Magic number validation (constant fields, e.g. `magic = xc0de`)

### Alignment & Padding

- [x] `@align(N)` - ensure field starts at N-byte boundary
- [x] `@pad(N)` - skip N bytes
- [x] `@pad_to(N)` - pad until offset is multiple of N
- [x] `@skip` / explicit field skipping

---

## Guarantees AI Cannot Provide

### No-Panic Guarantee

- [ ] All slice accesses bounds-checked at parse time
- [ ] No `.unwrap()` in generated code (use `?` propagation)
- [ ] Prove via `#[cfg(test)]` + `panic = "abort"` + fuzzing
- [ ] Document which error conditions are possible

### Compile-Time Verification

- [ ] Static offset computation when all fields are fixed-size
- [ ] Detect overlapping fields at codegen time
- [ ] Warn on suspicious patterns (e.g., length field after variable data)

### Fuzzing Integration

- [ ] Generate `Arbitrary` impl for each struct
- [ ] Generate `proptest` strategies
- [x] Roundtrip property: `parse(serialize(x)) == x` (writer round-trip tests + `generated_writers` fuzz target)
- [x] Crash-resistance property: any `&[u8]` either parses or returns error (`generated_parsers` fuzz target)

---

## Automation AI Cannot Maintain

### Serialization (Write Path)

- [x] Generate writer structs (`to_vec`, `encoded_len`)
- [x] Zerocopy in-place writer: `writer_over` (Mode 2)
- [x] Endianness-aware writing
- [x] Same hooks work for both parse and serialize (`@write_hook`)
- [x] Shipped for the full suite incl. MQTT v3/v5 + DNS name compression
- [ ] Content-range checksums/CRC/MAC (backpatch pass) — open frontier
- [ ] Forward typestate (compile-time-safe incremental) builder — open frontier

### Cross-Language Generation

- [ ] C header generation (`struct __attribute__((packed))`)
- [ ] Python `struct.unpack` generation
- [ ] TypeScript `DataView` generation
- [ ] Language-specific hook implementations

### Test Generation

- [ ] Generate valid packet examples from spec
- [ ] Generate edge cases (max lengths, boundary values)
- [ ] Generate invalid packets (truncated, bad magic, overflow)
- [ ] Snapshot tests for generated code

### Documentation Generation

- [ ] Markdown protocol documentation from DSL
- [ ] Field offset tables
- [ ] Packet diagrams (ASCII or SVG)
- [ ] Cross-reference between struct types

---

## Protocol Evolution Support

### Versioning

- [ ] `@since(version)` - field only present in version >= X
- [ ] `@deprecated(version)` - field removed in version >= X
- [ ] `@renamed(old_name, version)` - track renames
- [ ] Version-parameterized structs

### Diffing

- [ ] Compare two DSL specs, output changes
- [ ] Detect breaking changes (field removed, type changed)
- [ ] Detect compatible changes (field added at end)
- [ ] Migration guide generation

---

## Developer Experience

### Error Messages

- [ ] Source location in parse errors (field name, byte offset)
- [ ] "Expected X bytes for field `foo`, got Y"
- [ ] Hex dump of problematic region
- [ ] Suggest fixes for common mistakes

### IDE Support

- [ ] Tree-sitter grammar for syntax highlighting
- [x] LSP server (`binparse-lsp`) with multi-error parse recovery
- [x] Diagnostics (parse + codegen errors)
- [ ] Go-to-definition / hover info
- [ ] Auto-complete for attribute names

### Debug Visualization

- [ ] `#[derive(Debug)]` with meaningful output
- [ ] Hex view with field annotations
- [ ] Optional `Display` impl for human-readable output

---

## Performance

### Compile-Time Optimization

- [ ] Inline small field accessors
- [ ] Const-evaluate fixed offsets
- [ ] Avoid redundant bounds checks

### Runtime Optimization

- [ ] SIMD for array parsing where applicable
- [ ] Batch validation (check total length once upfront)
- [x] Zero-allocation iterators (already have this)
- [x] `@cache(len|value)` memoization to avoid redundant offset re-walks
- [x] Union parse-result caching

---

## Immediate Priorities

| Priority | Feature               | Status | Rationale                         |
| -------- | --------------------- | ------ | --------------------------------- |
| P0       | Hooks for VarInt      | done   | Unlocks MQTT, Protobuf, WebSocket |
| P0       | Consume-rest arrays   | done   | Unlocks variable payloads         |
| P1       | Serialization         | done   | Symmetry; proves spec is complete |
| P1       | No-panic guarantee    | partial (fuzzed) | Key differentiator from AI |
| P2       | Fuzzing integration   | partial (parser+writer targets) | Proves correctness at scale |
| P2       | Validation attributes | done   | Catches bad data early            |
| P3       | Checksums / typestate writer | open | Emit *valid* packets, not just well-formed |
| P3       | Cross-language        | open   | Multiplies value of each spec     |
| P3       | Documentation gen     | open   | Specs become source of truth      |
