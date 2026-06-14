use super::*;

fn generate_writers_err(dsl: &str) -> Error {
    let ast = binparse_dsl_parse::parse_str(dsl).expect("failed to parse DSL");
    CodeGen::generate_writers(&ast).expect_err("expected writer codegen to fail")
}

fn assert_generated_writers_eq(dsl: &str, snapshot: &str) {
    let actual = normalized_items(&generate_writers(dsl)).join("\n");
    let path = snapshot_path(snapshot);
    if std::env::var_os("BLESS").is_some() {
        std::fs::write(&path, format!("{actual}\n"))
            .unwrap_or_else(|e| panic!("failed to write snapshot {snapshot}: {e}"));
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("missing snapshot {snapshot}; rerun with BLESS=1 to regenerate")
    });
    let expected = expected.strip_suffix('\n').unwrap_or(&expected);
    if actual != expected {
        panic!(
            "generated writer code does not match snapshot {snapshot}; rerun with BLESS=1 to regenerate\n\n--- expected ---\n{expected}\n--- actual ---\n{actual}"
        );
    }
}

#[test]
fn writer_fixed_primitives() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Mixed { a: u8, @endian(little) b: u16, c: u32 }"#,
        "writer_fixed_primitives",
    );
}

#[test]
fn writer_bitfields() {
    assert_generated_writers_eq(
        r#"struct Ip { version: b<4>, ihl: b<4>, ttl: u8, total: u16 }"#,
        "writer_bitfields",
    );
}

#[test]
fn writer_byte_array() {
    assert_generated_writers_eq(
        r#"struct Eth { dst: [u8; 6], src: [u8; 6], ethertype: u16 }"#,
        "writer_byte_array",
    );
}

#[test]
fn writer_struct_ref() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Inner { a: u8, b: u16 } @endian(big) struct Outer { tag: u8, inner: Inner, trailer: u8 }"#,
        "writer_struct_ref",
    );
}

#[test]
fn writer_dynamic_tail() {
    assert_generated_writers_eq(
        r#"struct Frame { kind: u8, len: u8, payload: [u8; len] }"#,
        "writer_dynamic_tail",
    );
}

#[test]
fn writer_affine_dynamic_tail() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Udp { src: u16, dst: u16, length: u16, checksum: u16, payload: [u8; length - 8] }"#,
        "writer_affine_dynamic_tail",
    );
}

#[test]
fn writer_dynamic_region_with_trailer() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Frame { kind: u8, len: u8, payload: [u8; len], crc: u16, tail: u8 }"#,
        "writer_dynamic_region_with_trailer",
    );
}

#[test]
fn writer_varint_hook_tail() {
    assert_generated_writers_eq(
        r#"struct VarFrame {
            tag: u8,
            @hook(binparse.hooks.leb128_unsigned, u64) @write_hook(binparse.hooks.write_leb128_unsigned, binparse.hooks.leb128_unsigned_len) len: [u8],
            body: [u8; len],
        }"#,
        "writer_varint_hook_tail",
    );
}

#[test]
fn writer_len_region_struct_ref() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Inner { a: u8, b: u16 } @endian(big) struct Tlv { tag: u8, length: u8, @len(length) value: Inner, crc: u16 }"#,
        "writer_len_region_struct_ref",
    );
}

#[test]
fn writer_len_region_union() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Packet {
            kind: u8,
            length: u8,
            @len(length) body: union(kind) {
                1 => Connect { keep_alive: u16 },
                2 => Connack { ack: u8, code: u8 },
                _ => Unknown { },
            },
        }"#,
        "writer_len_region_union",
    );
}

#[test]
fn writer_union() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Packet {
            kind: u8,
            body: union(kind) {
                1 => Connect { keep_alive: u16 },
                2 => Connack { ack: u8, code: u8 },
                _ => Unknown { },
            },
        }"#,
        "writer_union",
    );
}

#[test]
fn writer_union_multi_literal() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Packet {
            kind: u8,
            body: union(kind) {
                1 | 2 => Hello { v: u16 },
                3 => Bye { code: u8 },
                _ => Unknown { },
            },
        }"#,
        "writer_union_multi_literal",
    );
}

#[test]
fn writer_union_tuple_discriminant() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Packet {
            a: u8,
            b: u8,
            body: union(a, b) {
                (0, 0) => Both { v: u16 },
                (1, 2) => OneTwo { code: u8 },
                _ => Unknown { },
            },
        }"#,
        "writer_union_tuple_discriminant",
    );
}

#[test]
fn writer_union_writable_wildcard() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Packet {
            kind: u8,
            body: union(kind) {
                1 => Connect { keep_alive: u16 },
                _ => Generic { a: u8, b: u8 },
            },
        }"#,
        "writer_union_writable_wildcard",
    );
}

#[test]
fn writer_union_error_arm() {
    assert_generated_writers_eq(
        r#"error { BadKind { got: u8 }, }
        @endian(big) struct Packet {
            kind: u8,
            body: union(kind) {
                1 => Connect { keep_alive: u16 },
                _ => @error(BadKind { got: kind }),
            },
        }"#,
        "writer_union_error_arm",
    );
}

#[test]
fn writer_union_dynamic_variant_skipped() {
    let code = generate_writers(
        r#"@endian(big) struct Rr {
            atype: u8,
            rdlength: u8,
            @len(rdlength) rdata: union(atype) {
                1 => A { addr: [u8; 4] },
                5 => Cname { @greedy(unsafe_eof) labels: [u8] },
                _ => Raw { @greedy(unsafe_eof) bytes: [u8] },
            },
        }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(!code.contains("RrWriter"));
}

#[test]
fn writer_counted_array_of_structs() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Pair { a: u8, b: u16 } @endian(big) struct Rec { n: u8, items: [Pair; n], crc: u16 }"#,
        "writer_counted_array_of_structs",
    );
}

#[test]
fn writer_greedy_array_of_structs() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Pair { a: u8, b: u16 } @endian(big) struct Rec { tag: u8, @greedy(unsafe_eof) items: [Pair] }"#,
        "writer_greedy_array_of_structs",
    );
}

#[test]
fn writer_greedy_open_tail() {
    assert_generated_writers_eq(
        r#"struct Vlan {
            dst: [u8; 6],
            src: [u8; 6],
            tpid = x8100,
            pcp: b<3>,
            dei: b<1>,
            vid_hi: b<4>,
            vid_lo: u8,
            ethertype: u16,
            @greedy(unsafe_eof) payload: [u8],
        }"#,
        "writer_greedy_open_tail",
    );
}

#[test]
fn writer_conditional_with_else() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Cond {
            n: u8,
            if (n == 1) {
                x: u16,
            } else {
                y: u8,
            }
        }"#,
        "writer_conditional_with_else",
    );
}

#[test]
fn writer_conditional_no_else() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Opt {
            flags: u8,
            if (flags > 0) {
                a: u16,
                b: u8,
            }
        }"#,
        "writer_conditional_no_else",
    );
}

#[test]
fn writer_conditional_not_last_skipped() {
    let code = generate_writers(
        r#"struct Foo {
            n: u8,
            if (n == 1) {
                x: u16,
            }
            tail: u8,
        }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(!code.contains("FooWriter"));
}

#[test]
fn writer_conditional_dynamic_branch_skipped() {
    let code = generate_writers(
        r#"struct Bar {
            n: u8,
            len: u8,
            if (n == 1) {
                payload: [u8; len],
            }
        }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(!code.contains("BarWriter"));
}

#[test]
fn writer_hook_len_without_write_hook_errors() {
    let err = generate_writers_err(
        r#"struct VarFrame {
            tag: u8,
            @hook(binparse.hooks.leb128_unsigned, u64) len: [u8],
            body: [u8; len],
        }"#,
    );
    assert!(
        matches!(
            err,
            Error::Writer(crate::writer::Error::MissingWriteHook { .. })
        ),
        "expected MissingWriteHook, got {err:?}"
    );
    assert!(err.to_string().contains("write_hook"));
}

#[test]
fn writer_content_hook() {
    assert_generated_writers_eq(
        r#"struct Msg {
            count: u8,
            @hook(binparse.hooks.length_prefixed_bytes, &'a [u8]) @write_hook(binparse.hooks.write_length_prefixed_bytes, binparse.hooks.length_prefixed_bytes_len) body: [u8],
        }"#,
        "writer_content_hook",
    );
}

#[test]
fn writer_content_hook_without_width_skipped() {
    let code = generate_writers(
        r#"struct Msg {
            count: u8,
            @hook(binparse.hooks.length_prefixed_bytes, &'a [u8]) @write_hook(binparse.hooks.write_length_prefixed_bytes) body: [u8],
        }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(!code.contains("MsgWriter"));
}

#[test]
fn writer_content_hook_without_write_hook_errors() {
    let err = generate_writers_err(
        r#"struct Msg {
            count: u8,
            @hook(binparse.hooks.length_prefixed_bytes, &'a [u8]) body: [u8],
        }"#,
    );
    assert!(
        matches!(
            err,
            Error::Writer(crate::writer::Error::MissingWriteHook { .. })
        ),
        "expected MissingWriteHook, got {err:?}"
    );
}

#[test]
fn writer_mid_struct_union_skipped() {
    let code = generate_writers(
        r#"@endian(big) struct Packet {
            kind: u8,
            body: union(kind) {
                1 => Connect { keep_alive: u16 },
                _ => Unknown { },
            },
            trailer: u8,
        }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(!code.contains("PacketWriter"));
}

#[test]
fn writer_until_tail_skipped() {
    let code = generate_writers(
        r#"struct Listing { count: u8, @until(x00) entries: [u8] }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(!code.contains("ListingWriter"));
}

#[test]
fn writer_multibyte_array() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Samples { count: u8, data: [u16; 4], crc: u16 }"#,
        "writer_multibyte_array",
    );
}

#[test]
fn writer_multibyte_array_little_endian() {
    assert_generated_writers_eq(
        r#"struct LeSamples { @endian(little) data: [u32; 2] }"#,
        "writer_multibyte_array_little_endian",
    );
}

#[test]
fn writer_multibyte_array_dynamic_count_skipped() {
    let code = generate_writers(r#"struct Foo { n: u8, data: [u16; n] }"#);
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(!code.contains("FooWriter"));
}

#[test]
fn writer_concat() {
    assert_generated_writers_eq(
        r#"@endian(big) struct WithConcat { tag: u8, combo: concat(u8, u16), trailer: u8 }"#,
        "writer_concat",
    );
}

#[test]
fn writer_concat_bitfield_nibbles() {
    assert_generated_writers_eq(
        r#"struct Nib { combo: concat(b<4>, b<4>), tail: u8 }"#,
        "writer_concat_bitfield_nibbles",
    );
}

#[test]
fn writer_concat_subbyte_start_skipped() {
    let code = generate_writers(
        r#"struct Foo { flags: b<3>, fragment_offset: concat(b<5>, u8) }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(!code.contains("FooWriter"));
}

#[test]
fn writer_concat_multibyte_item() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Mix { tag: u8, combo: concat([u8; 2], u16), trailer: u8 }"#,
        "writer_concat_multibyte_item",
    );
}

#[test]
fn writer_concat_struct_ref_item_skipped() {
    let code = generate_writers(
        r#"struct Inner { a: u8 } struct Foo { combo: concat(u8, Inner) }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(!code.contains("FooWriter"));
}

#[test]
fn writer_pad() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Padded { a: u8, @pad(2) b: u16, c: u8 }"#,
        "writer_pad",
    );
}

#[test]
fn writer_pad_to() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Aligned { a: u8, @pad_to(4) b: u32 }"#,
        "writer_pad_to",
    );
}

#[test]
fn writer_align() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Al { a: u16, @align(2) b: u16 }"#,
        "writer_align",
    );
}

#[test]
fn writer_skip() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Sk { a: u8, @skip reserved: u16, b: u8 }"#,
        "writer_skip",
    );
}

#[test]
fn writer_struct_len_skipped() {
    let code = generate_writers(r#"@len(8) struct Foo { a: u8, b: u16 }"#);
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(!code.contains("FooWriter"));
}

#[test]
fn writer_over_dynamic_region() {
    assert_generated_writers_eq(
        r#"@endian(big) struct Msg { kind: u8, len: u8, body: [u8; len], crc: u16 }"#,
        "writer_over_dynamic_region",
    );
}

#[test]
fn writer_over_union_not_emitted() {
    let code = generate_writers(
        r#"@endian(big) struct Packet {
            kind: u8,
            body: union(kind) {
                1 => Connect { keep_alive: u16 },
                2 => Connack { ack: u8 },
                _ => Unknown { },
            },
        }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    let items = normalized_items(&code);
    let packet_impl = items
        .iter()
        .find(|item| item.contains("impl") && item.contains("PacketWriter"))
        .expect("PacketWriter impl should be emitted");
    assert!(!packet_impl.contains("writer_over"));
}
