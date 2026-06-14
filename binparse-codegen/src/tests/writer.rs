use super::*;

#[test]
fn writer_for_simple_fixed_struct() {
    let code = generate_writers(r#"@endian(big) struct P { a: u8, b: u16, c: u32 }"#);
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(code.contains("PWriter"));
    assert!(code.contains("PContent"));
}

#[test]
fn writer_for_bitfield_struct() {
    let code = generate_writers(r#"struct Ip { version: b<4>, ihl: b<4>, ttl: u8, total: u16 }"#);
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(code.contains("IpWriter"));
    assert!(code.contains("IpContent"));
    assert!(code.contains("set_version"));
}

#[test]
fn writer_for_byte_array_struct() {
    let code = generate_writers(r#"struct Eth { dst: [u8; 6], src: [u8; 6], ethertype: u16 }"#);
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(code.contains("EthWriter"));
    assert!(code.contains("EthContent"));
    assert!(code.contains("dst_mut"));
}

#[test]
fn writer_for_struct_ref_struct() {
    let code = generate_writers(
        r#"@endian(big) struct Inner { a: u8, b: u16 } @endian(big) struct Outer { tag: u8, inner: Inner, trailer: u8 }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(code.contains("OuterWriter"));
    assert!(code.contains("inner_mut"));
    assert!(code.contains("InnerContent"));
}

#[test]
fn writer_for_dynamic_tail_struct() {
    let code = generate_writers(r#"struct Frame { kind: u8, len: u8, payload: [u8; len] }"#);
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(code.contains("FrameWriter"));
    assert!(code.contains("FrameLens"));
    assert!(code.contains("FrameContent"));
    assert!(code.contains("payload_mut"));
    assert!(!code.contains("set_len"));
}

#[test]
fn writer_for_varint_dynamic_tail_struct() {
    let code = generate_writers(
        r#"struct VarFrame {
            tag: u8,
            @hook(binparse.hooks.leb128_unsigned, u64) @write_hook(binparse.hooks.write_leb128_unsigned, binparse.hooks.leb128_unsigned_len) len: [u8],
            body: [u8; len],
        }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(code.contains("VarFrameWriter"));
    assert!(code.contains("len_width"));
    assert!(code.contains("body_mut"));
    assert!(!code.contains("set_len"));
}

#[test]
fn writer_for_ethernet_greedy_payload() {
    let code = generate_writers(
        r#"struct EthernetII {
    dst: [u8; 6],
    src: [u8; 6],
    @discriminator ethertype: u16,
    @greedy(unsafe_eof) @payload payload: [u8],
}"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(code.contains("EthernetIIWriter"));
    assert!(code.contains("EthernetIIContent"));
    assert!(code.contains("payload_mut"));
    assert!(code.contains("set_ethertype"));
}

#[test]
fn writer_for_vlan_constant_field() {
    let code = generate_writers(
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
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(code.contains("VlanWriter"));
    assert!(code.contains("VlanContent"));
    assert!(code.contains("payload_mut"));
    assert!(code.contains("set_pcp"));
    assert!(!code.contains("set_tpid"));
}

#[test]
fn writer_for_union_struct() {
    let code = generate_writers(
        r#"@endian(big) struct Packet {
            kind: u8,
            body: union(kind) {
                1 => Connect { keep_alive: u16 },
                2 => Connack { ack: u8, code: u8 },
                _ => Unknown { },
            },
        }"#,
    );
    syn::parse_str::<syn::File>(&code).expect("generated writer code is not valid Rust");
    assert!(code.contains("PacketWriter"));
    assert!(code.contains("PacketContent"));
    assert!(code.contains("PacketBodyContent"));
    assert!(code.contains("ConnectContent"));
    assert!(code.contains("ConnectWriter"));
    assert!(!code.contains("set_kind"));
}
