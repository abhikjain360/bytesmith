# Efficient Network Packet Parser Generator

A declarative DSL which compiles to efficient Rust code.

### 1. Data Layout & Primitives

These requirements define how the DSL must describe the physical arrangement of bits and bytes in memory.

- **R1.1 Bit-Level Granularity:** The DSL must support defining bit-slice fields with arbitrary bit-widths (e.g., 1-bit flags, 3-bit reserved fields, 20-bit flow labels).
    - Bit-slice fields have type `b<N>` where `N` is the number of bits.
    - Max `N` for a single bit-field is 128.
    - Bit-slice getters return the smallest unsigned integer primitive that can hold `N` bits (e.g., `b<1>`-`b<8>` return `u8`, `b<9>`-`b<16>` return `u16`, etc.).
    - Signed bit-fields are not supported.
    - **R1.1.1 Bit-ordering Across Bytes:** Bit numbering within a byte follows the specified endianness/bit-order (R1.2), and multi-byte bitfields are reconstructed by concatenating bits in the order they appear in the stream.
- **R1.2 Explicit Endianness:** The DSL must allow specifying endianness (Big or Little, default to Big as is default in most protocols) at three levels:
    - **R1.2.1 Global default:** (e.g., Network Byte Order).
    - **R1.2.2 Per-struct override.**
    - **R1.2.3 Per-field override:** (e.g., a Little-Endian field inside a Big-Endian protocol).
Bit-level ordering (LSB 0 vs MSB 0) must also be definable, defaulting to MSB 0 (most common in network protocols).
- **R1.3 Primitive Type Support:** The DSL must map to unsigned integer primitives (`u8` through `u128`) and fixed-size byte arrays (`[u8; 6]`).
    - There is no `bool` type; use `b<1>` for single-bit flags.
    - Signed integer primitives are not supported in the DSL (use unsigned + user-side interpretation if needed).
    - **R1.3.1 Unaligned Access:** The generated code must handle unaligned memory access safely and efficiently (e.g., using `read_unaligned` or `from_be_bytes`), as network buffers are not guaranteed to be aligned to the host's word boundaries.
- **R1.4 Padding & Alignment:** The DSL must support declarative "skip" or "reserved" fields that advance the bit/byte cursor without exposing a value in the public API.
    - **R1.4.1 Alignment Constraints:** The DSL must allow requiring byte-alignment for a field (common for byte arrays and nested packets). If a non-byte-aligned cursor attempts to start a byte-aligned field, parsing must fail.
    - **R1.4.2 Cursor Units:** The internal layout cursor is tracked in **bits**, so bit-fields compose correctly.
    - **R1.4.3 Bitfield Grouping Rule:** A sequence of `b<N>` fields that conceptually represents a byte (e.g., an 8-bit flags byte split into multiple bit fields) must be explicitly padded/skipped so that the cursor returns to a byte boundary (multiple of 8 bits) before any subsequent non-`b<N>` field. Violations are DSL-level errors.
- **R1.5 Field as Concatenation of disjoint bits:** A field can be something like `field: concat(chunk_a: b<4>, skip<b<4>>, chunk_b: b<8>)`. In this case the alignment rule for bitifield groupings considers the field itself a list of bit fields.
- **R1.6 Unions / Choices:** The DSL must allow defining a choice between multiple layouts at the same cursor position.
    - **R1.6.1 Selection Rule:** Union selection must be *deterministic* based on an explicit condition/guard (e.g., `if kind == 1 => VariantA`).
    - **R1.6.2 Multi-Field Selection (Tuple Unions):** The DSL must support matching on a tuple of fields (e.g., `if (major, minor) == (1, 0) => V1_0`). This prevents nested unions when multiple fields determine the variant.
    - **R1.6.3 No Lookahead:** The discriminant/guard must be determined solely by fields parsed *prior* to the union. "Peeking" into the union's own data to determine its type is not supported.
    - **R1.6.4 Fallback Rule:** If a union variant has no guard, it is treated as a fallback. A fallback MUST be provided if not all variants of gaurd can be matched upon.
    - **R1.6.5 Cursor Advancement:** A union is a **choice**, not an overlay: the cursor advances by the selected variant's size.
        - If all variants are fixed-length and equal-sized, the union is fixed-length.
        - Otherwise, the union is dynamic and must participate in boundary caching (R5.6).
    - **R1.6.6 Validation Rule:** A variant "matches" only if it passes its own bounds checks and semantic validations.
- **R1.7 Bit Literals & Constants:** The DSL must support using binary literals (e.g., `b110`), hexadecimal literals (e.g., `xFF`), and decimal literals to represent bit values in expressions, conditions, and field constraints.
    - **R1.7.1 Constant Constraints:** Fields can be constrained to a specific constant value (e.g., `magic: b<4> = b1010`). The parser must validate that the input matches this value; otherwise, it is a parse error.
    - **R1.7.2 Width Validation:** The DSL must verify at compile-time that a bit literal fits within the target `b<N>` width.

### 2. Variable Length & Dynamic Sizing

These requirements handle fields where the size is not known until runtime.

- **R2.1 Length-Prefixed Fields:** The DSL must support fields whose size is determined by the _value_ of a previous integer field (e.g., `payload` length defined by `total_length` field). This "previous integer field" can be any field that appeared earlier in parsing, that is, defined earlier in the struct.
- **R2.2 Expression-Based Sizing:** The definition of size must support arithmetic expressions involving previous fields and constants (e.g., `length = (offset_field * 4) - 20`).
    - Expressions are evaluated in strict declaration order and may only reference fields defined earlier in the same struct (or values explicitly passed via context propagation).
- **R2.3 Self-Describing Encodings (VarInts):** The DSL must support fields where the size is determined by the bit-pattern of the data itself (e.g., MQTT/Protobuf VarInts). This means that the DSL must allow "hooks" which are functions that user is responsible for defining, e.g. via an attribute like `@parse_with` specifying `"crate::var_int::parser"`.
    - The DSL does not have a string type; decoded outputs are integers and/or bit/byte slices.
- **R2.4 Sentinel-Terminated Fields:** The DSL must support fields that read until a specific byte value is found (e.g., reading bytes until `0x00`).
    - **Termination Policy:** Sentinel searches must stop immediately if the end of the available buffer is reached.
    - **Security Bounds:** The DSL should allow specifying a `max_scan` limit for sentinel-terminated fields to prevent DOS on malformed input.
    - **Vectorization Hints:** The DSL should allow hinting that a field is expected to be long/text, triggering SIMD scanner generation.
    - **R2.4.1 Optimization:** For byte-based sentinels, the generator must utilize optimized search routines (e.g., `memchr` or SIMD intrinsics) where available.
- **R2.5 End-of-Input Consumers:** The DSL must support a "Greedy" type.
    - **Safety Constraint:** To prevent accidental consumption of buffer padding/garbage, defining a `@greedy` field is a **DSL compilation error** unless a scope limit has been explicitly established.
        - **Scope Establishment:** A scope is established either by an internal field (via `@len` on struct, see R2.8) or by the parent protocol providing a length context (see R4.3).
    - **Unsafe Override:** The DSL must provide a mechanism (e.g., `@greedy(unsafe_eof)`) for users to explicitly opt-in to consuming the physical remainder of the buffer. This is intended for top-level streams or where the physical buffer end is the known logical end.
    - **Positioning Rule:** A `@greedy` (or `@greedy(unsafe_eof)`) field must be the *last* field in its struct.
- **R2.6 Dynamic Field Boundary Rules:** For every variable-length field, the parser must be able to compute its end boundary during the constructor scan.
    - If a dynamic field is not the last field, its end boundary is the start of the next *present* field.
    - If a dynamic field is the last field, it must be either Greedy, sentinel-terminated, or length-specified; otherwise it is a spec error.
- **R2.7 Opaque Fields:** The DSL must support an `@opaque` attribute for types.
    - The constructor calculates the total size of `T` (skipping over it) and stores the boundary offsets.
    - It performs **no** validation of `T`'s internal structure (fields, invariants, inner variants) during the scan.
    - Validation and parsing of `T` only occur when the field is explicitly accessed/unwrapped. This allows high-performance skipping of complex sub-structures.
- **R2.8 Self-Limiting Scopes:** The DSL must allow a struct or field to define the logical end of its data (e.g., via `@len(total_len)`).
    - **R2.8.1 Struct-level `@len`:** Restricts the parsing scope for all fields in the struct to the specified range/length. The length is determined by a field within the struct.
    - **R2.8.2 Field-level `@len`:** Restricts the parsing scope for a specific field (e.g. a nested struct) to the specified range/length.
    - Bytes beyond this scope are ignored by the current parser (but remain available to the caller/parent).

### 3. Conditional Presence & Optionality

Network protocols often have fields that only exist if a flag is set.

- **R3.1 Bit-Flag Dependency:** The DSL must allow a field's existence to depend on the value of a specific bit in a previous field (e.g., `if flags.has_syn { ... }`).
- **R3.2 Value-Based Dependency:** The DSL must allow fields to be conditional based on enum variants or integer values of prior fields (e.g., `if type == 5 { ... }`).
- **R3.3 Repeated Fields (Arrays):** The DSL must offer a mechanism to define lists of fields (`[T; N]`).
    - **R3.3.1 Count-Based Arrays:** Arrays must be count-based, where `N` is either a compile-time constant or determined by a previously-parsed integer field.
    - **R3.3.2 Max iterations (BPF Mode):** If generating code for BPF targets, dynamic arrays must have a `max_iteration` limit even if the dynamic count is lower, ensuring the BPF verifier can unroll or bound the loop.
    - **R3.3.3 BPF Loop Unrolling:** For BPF targets, if max_iter is small (e.g., < 16), the generator should unroll the loop. If larger, it must generate a distinct for loop with a hard break: `if i >= MAX_ITER { break; }`.
- **R3.4 Allocation-Free Iterators:** Arrays must be representable without heap allocation. The generated code should expose them as a zero-copy view/iterator over the backing bytes.
    - The constructor scan must validate that iterating `count` elements stays within bounds (and respects any logical end supplied via context propagation).
- **R3.5 Optional Field Layout Semantics:** Optional fields must have well-defined effects on subsequent offsets.
    - The constructor scan must decide presence/absence and compute the next cursor position accordingly.
    - The parsed struct must store enough information (e.g., a bitset of presences and boundary offsets) so that getters for later fields remain O(1).

### 4. Composition & Encapsulation

These requirements define how independent header definitions are glued together into a protocol stack.

- **R4.1 Protocol Graph Definition:** The DSL must provide a mechanism to define transitions between protocols (e.g., "If Ethernet.type == 0x0800, next is IPv4").
    - Protocol transitions must be representable as a tagged choice, selected by conditions over already-parsed fields.
    - Next-protocol parsing must remain lazy (selected and parsed only when accessed).
- **R4.2 Offset Inheritance (Constant Folding):** When Protocol B is nested in Protocol A, the DSL must mandate that B's base offset is calculated relative to A's base. If A has fixed size, B's offsets must compile to simple additions (no runtime lookups).
- **R4.3 Context Propagation & Bounded Parsing:** The DSL must allow passing data from the parent protocol to the child.
    - **R4.3.1 Logical End Constraints:** If a parent protocol supplies a `total_length` or `limit` via context, the child parser must strictly treat that boundary as the end-of-buffer, ignoring any actual trailing bytes in the physical slice (to handle padding correctly).
    - **R4.3.2 Mandatory Scope Propagation:** If a child struct contains a `@greedy` field (and lacks an internal `@len` on the struct), the parent **must** provide a length argument when instantiating that child. Failure to do so is a DSL compilation error.
    - Failure to provide sufficient bytes to satisfy a child's context-defined limit must be a parse error.
- **R4.4 Encapsulation Agnosticism:** The DSL must define headers independently of their position. (e.g., An "IP" header definition should be reusable whether it is inside Ethernet or inside an IP-in-IP tunnel).
- **R4.5 Cross-Layer Constraints:** The DSL must allow defining consistency checks that span across protocol boundaries. Example: A rule that asserts Parent.payload_length == Child.length. These checks should be enforced during the "lazy" access of the child protocol, returning a validation error if the cross-layer logic is violated.

### 5. Performance & Memory Model (The "Zero-Copy" Constraint)

These constraints dictate the structure of the generated Rust code.

- **R5.1 Reference-Only Storage:** The generated structs must strictly hold references (`&[u8]`) and offsets (`usize`). They must strictly forbid heap allocation (`Vec`, `String`) in the hot path.
- **R5.2 Lazy Evaluation:** Field parsing (endian swapping, bit masking) must happen only when the getter method is called, not during struct instantiation.
    - Exception: the constructor scan may evaluate expressions required for sizing/presence decisions and boundary computation.
- **R5.3 Single-Pass Validation (Shallow):** The generated constructor must perform a linear scan of the *current* layer to calculate dynamic offsets and validate bounds. It should *not* recursively validate the contents of nested protocol payloads (encapsulated packets) until they are accessed.
- **R5.4 Bounds Check Hoisting:** Where possible, the generated code must perform bounds checks at the "Header" level (e.g., "do we have 20 bytes?") rather than the "Field" level, allowing individual field getters to have unchecked/unsafe variants to which may be used internally/externally for speed.
- **R5.5 Lifetime Management:** All generated structs must accept a lifetime parameter `'a` (e.g., `Packet<'a>`) that ties the struct to the input `&'a [u8]` slice.
- **R5.6 Offset Caching & Relative Addressing:** For packets with variable-length or optional fields, the parsed struct must store the calculated *boundary* offsets needed to make later getters O(1). Subsequent fixed-width fields must use relative offsets from these stored boundaries (e.g., `boundary_after_options + 4`) to minimize storage overhead.
    - **R5.6.1 Opt-out Caching:** The DSL must allow users to explicitly opt-out of offset caching for specific dynamic fields. In this case, the offset is recalculated on-the-fly during access, trading CPU for reduced struct size.
    - **R5.6.2 Variable-Size Length Fields:** The parser scan loop must handle fields where the size indicator itself is variable-length (e.g., MQTT VarInts). It must utilize the R2.3 hook's return value (value, bytes_consumed) to skip the variable-length size field before calculating the subsequent payload boundary.
- **R5.7 Presence & Boundary Encoding:** The generator should prefer compact representations:
    - Presence flags stored as bitsets when possible.
    - Only store boundaries that are required to address later fields (avoid per-field offsets when constant folding can derive them).
- **R5.8 Constant Size Optimization:** If a defined struct and all its nested fields/children are statically fixed-size:
    - The generated parser must bypass field-by-field scanning.
    - It must compile down to a single bounds check (`buffer.len() >= sizeof(T)`) followed by unaligned-safe reading (e.g. `ptr::read_unaligned` or byte array mapping). It must avoid direct pointer casting (`&T`) unless alignment is statically proven.

### 6. Developer Interface & Extensibility

How the user interacts with the generated code.

- **R6.1 Pure Rust Hooks:** The DSL must allow the user to provide Rust hooks via attributes for:
    - Custom transformations (Decryption or Decompression) using `@transform`.
    - Custom parsers for self-describing encodings (used by R2.3) using `@parse_with`.
    - Custom validation logic (e.g. Checksums).
    - **R6.1.1 Hook Contract:** Hooks must be callable without allocating in the common case, must be able to operate on borrowed input, and must report how many bits/bytes they consumed. Hooks must never panic; errors must be reportable to the caller.

- **R6.2 Safe & Unsafe APIs:** The generated code should expose a Safe API (returns `Result/Option`) and arguably an Unchecked API for users who have pre-validated the data.
- **R6.3 Debugging Support:** The generated structs must implement `Debug` traits that pretty-print the interpreted packet fields, not just the raw bytes.

### 7. Safety

- **R7.1 Malformed Packet Handling:** The DSL must define behavior for truncation (packet ends mid-field). It must guarantee that no panic occurs on malformed input.
- **R7.2 Semantic Validation:** The DSL should allow defining valid ranges/sets for fields (e.g., `version must be 4`) and treat values outside this range as a parsing error (or expose them as "Unknown").
- **R7.3 Arithmetic Safety:** Size/offset expressions must be checked for overflow/underflow. Any invalid arithmetic (e.g., negative length, wraparound) must be a parse error.
- **R7.4 DSL-Level Type/Option Safety:** Referencing a field that may be absent (optional) inside a size/presence expression must be made explicit via DSL helpers (e.g., `throw_on_none(optional_field)` or equivalent).
    - If an expression references an optional field without an explicit unwrap/error strategy, it is a DSL-level error (compile-time), not a runtime parse error.
- **R7.5 Verifier Constraints:** generated code may optionally use `unreachable_unchecked` or assertions that help the verifier understand ranges.

### 8. Custom Error Types

- **R8.1 Unified Error Type:** The DSL must allow defining a single root-level `error` block that specifies custom error variants.
- **R8.2 Primitive Payloads:** Custom error variants can only contain primitive DSL types (`u8`..`u128`, `b<N>`). Complex types (structs, arrays) are not supported to keep error types lightweight and copyable.
- **R8.3 Automatic Variants:** The generated Rust error enum must automatically include `Io(std::io::Error)` to handle underlying reader errors, along with standard parsing errors (e.g. `UnexpectedEof`).

### 9. Edge Cases (The "Gaps")

- **R9.1 Memory Model (Linear Buffers Only):** To satisfy the `&[u8]` constraint (R5.1), the DSL will explicitly support *only* contiguous linear buffers. Fragmented packets (e.g., `iovec` or `sk_buff` chains) must be linearized by the caller before parsing.
- **R9.2 Bit-ordering vs Byte-ordering:** The DSL must clarify if bits are populated LSB-first or MSB-first within a byte. Default is MSB-first (Network Order).
- **R9.3 Recursive & Nested Protocols:**
    - **R9.3.1 Flattened Execution:** The generated code must strictly adhere to Lazy Evaluation (R5.2). Parent::parse must never automatically invoke Child::parse. It must only identify the payload boundaries. This ensures stack depth remains constant, satisfying BPF limits.
    - **R9.3.2 Manual Layering:** Handling of encapsulated packets (e.g., IP-in-IP) is the responsibility of the caller, effectively flattening the recursion loop.
- **R9.4 Zero-Sized Fields:** Handling of fields that resolve to 0 length (e.g. empty byte/bit slices) must be robust.
- **R9.5 Maximum Scan Work:** The constructor scan must have a well-defined upper bound for sentinel-terminated fields and repeats (e.g., stop at packet end; optionally allow user-defined max iterations) to avoid pathological inputs causing excessive work.
- **R9.6 BPF/No-Std Compliance:**
    - **R9.6.1 Panic-Free Access:** When compiling for #[no_std] or BPF targets, the generator must strictly avoid slice indexing ([]) and unwrap(). It must use .get(start..end).ok_or(Error::UnexpectedEof)? for all data access to ensure the BPF verifier never sees a potential panic path.
    - **R9.6.2 No Complex Iterators:** For BPF targets, variable-length field iteration (R3.3) must be generated as explicit for loops with hard break limits to satisfy the verifier.