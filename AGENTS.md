- rust codebase, edition 2024, resolver 3
- ast definitions in binparse-dsl
  - it is always recommended to read the binparse-dsl/src/lib.rs as it has the AST definition which you'll probably need for most tasks
- string to ast parser in biparse-dsl-parse
- common lib in binparse
- codgen in binparse-codegen
  - everywhere `use binparse_dsl as ast;`
- run codegen test using cargo run -p binparse-codegen --example test
- make sure to run cargo clippy --all-targets at the end
- as this codebase is complex, do only one step at a time, only do what is asked for and no more and ask for clarifications if doing what is asked for is not enough according to your analysis.
  - in the same spirit prefer using match arms with todo!() for other cases
- bitfield width is always less than 8
- don't leave comments, at all, unless asked otherwise
- keep the coding style consistent, read code for other places on how we handle different problems and follow the same pattern

## Codegen Architecture

The code generation flows through these layers:

```
lib.rs → struct_.rs → field.rs → type_/mod.rs → type_/*.rs
```

### Layer Responsibilities

| File | Input | Output | Generates |
|------|-------|--------|-----------|
| `lib.rs` | `Vec<ast::Definition>` | `String` (final Rust code) | Dependency resolution, coordinates struct gen, pretty-prints |
| `struct_.rs` | `ast::Struct` | `GeneratedStruct { len, tokens }` | Full struct def + impl block with `parse()` fn |
| `field.rs` | `ast::Field` | Populates `FieldAccum` | Field getter fn, `*_end_offset()` fn, helper fns |
| `type_/mod.rs` | `ast::Type` | `GeneratedTypeInfo` | Dispatches to specific type generators |
| `type_/*.rs` | Specific type AST | `GeneratedTypeInfo` | Getter body, length, return type |

### Key Data Structures

- `StructAccum`: Accumulates tokens while generating a struct (field defs, functions, offset tracking)
- `FieldAccum`: Accumulates tokens for a single field (getter, offset getter, helpers)
- `GeneratedTypeInfo`: What type generators return (len, getter body, return type, field type)
- `GeneratedLen`: Either `Fixed(Len)` for compile-time known or `Dynamic(TokenStream)` for runtime

### What Gets Generated Where

| Component | Generated In | Example |
|-----------|--------------|---------|
| `struct Foo<'a> { data: &'a [u8] }` | `struct_.rs` | Struct definition |
| `impl<'a> Foo<'a> { ... }` | `struct_.rs` | Impl block wrapper |
| `pub fn parse(data) -> Result<...>` | `struct_.rs` | Parse function |
| `pub fn field_name(&self) -> T { body }` | `field.rs` | Field getter fn signature |
| `pub fn field_end_offset(&self) -> Len` | `field.rs` | Offset getter fn |
| `self.data[0..4].try_into()...` | `type_/primitive.rs` | Getter body for primitives |
| `Iterator` structs for arrays | `type_/array.rs` | Goes into `other_entities` |
| Union enum + variant structs | `type_/union_.rs` | Goes into `other_entities` |

### Flow Example

For `struct Packet { len: u8, data: [u8; len] }`:

1. `lib.rs`: Resolves dependencies, calls `struct_::generate`
2. `struct_.rs`: Creates `StructAccum`, iterates fields, builds final tokens
3. `field.rs` (for `len`): Calls `type_::generate`, builds getter + offset getter
4. `type_/primitive.rs`: Returns `GeneratedTypeInfo` with `self.data[0]` body
5. `field.rs` (for `data`): Calls `type_::generate` for array
6. `type_/array.rs`: Creates iterator struct, returns `GeneratedTypeInfo`
7. `struct_.rs`: Assembles all tokens, inserts into `done` map
8. `lib.rs`: Pretty-prints final code
