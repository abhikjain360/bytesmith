use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn generated_code(dsl: &str) -> String {
    let ast = binparse_dsl_parse::parse_str(dsl).expect("failed to parse DSL");
    binparse_codegen::CodeGen::generate_writers(&ast).expect("failed to generate code")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("binparse-codegen should have a workspace parent")
        .to_path_buf()
}

fn write_runtime_crate(name: &str, code: &str, test_body: &str) -> PathBuf {
    let root = workspace_root();
    let test_dir = root
        .join("target")
        .join("writer-protocol-suite-tests")
        .join(format!("runtime-{}-{}", name, std::process::id()));

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(test_dir.join("src")).expect("failed to create runtime test crate");

    let binparse_path = root.join("binparse");
    fs::write(
        test_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "writer-protocol-suite-test"
version = "0.0.0"
edition = "2024"

[dependencies]
binparse = {{ path = "{}" }}

[workspace]
"#,
            binparse_path.display()
        ),
    )
    .expect("failed to write runtime Cargo.toml");

    fs::write(
        test_dir.join("src/lib.rs"),
        format!(
            r#"
{code}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn round_trip() {{
        {test_body}
    }}
}}
"#
        ),
    )
    .expect("failed to write runtime lib.rs");

    test_dir
}

fn run_round_trip(name: &str, dsl: &str, test_body: &str) {
    let code = generated_code(dsl);
    let test_dir = write_runtime_crate(name, &code, test_body);
    let output = Command::new("cargo")
        .arg("test")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(test_dir.join("Cargo.toml"))
        .output()
        .expect("failed to run generated runtime tests");

    assert!(
        output.status.success(),
        "generated runtime tests failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

const ETHERNET_DSL: &str = r#"
struct EthernetII {
    dst: [u8; 6],
    src: [u8; 6],
    @discriminator ethertype: u16,
    @greedy(unsafe_eof) @payload payload: [u8],
}
"#;

const VLAN_DSL: &str = r#"
struct Vlan {
    dst: [u8; 6],
    src: [u8; 6],
    tpid = x8100,
    pcp: b<3>,
    dei: b<1>,
    vid_hi: b<4>,
    vid_lo: u8,
    ethertype: u16,
    @greedy(unsafe_eof) payload: [u8],
}
"#;

const TCP_DSL: &str = r#"
struct TcpOption {
    kind: u8,
    body: union(kind) {
        0 => Eol { },
        1 => Nop { },
        _ => Generic { len: u8, data: [u8; len - 2] },
    },
}

struct TcpOptionList {
    @greedy(unsafe_eof) @max_iter(40) opts: [TcpOption],
}

struct Tcp {
    @discriminator src_port: u16,
    @discriminator dst_port: u16,
    seq: u32,
    ack_no: u32,
    @range(5, 15) data_offset: b<4>,
    reserved: b<3>,
    ns: b<1>,
    cwr: b<1>,
    ece: b<1>,
    urg: b<1>,
    ack: b<1>,
    psh: b<1>,
    rst: b<1>,
    syn: b<1>,
    fin: b<1>,
    window: u16,
    checksum: u16,
    urgent_ptr: u16,
    @len((data_offset - 5) * 4) options: TcpOptionList,
    @greedy(unsafe_eof) @payload payload: [u8],
}
"#;

const IP_DSL: &str = r#"
@len(total_len)
struct Ipv4 {
    @check(version == 4) version: b<4>,
    @range(5, 15) ihl: b<4>,
    dscp: b<6>,
    ecn: b<2>,
    @range(20, 65535) total_len: u16,
    ident: u16,
    flags: b<3>,
    frag_hi: b<5>,
    frag_lo: u8,
    ttl: u8,
    @discriminator proto: u8,
    checksum: u16,
    src: [u8; 4],
    dst: [u8; 4],
    if (ihl > 5) {
        options: [u8; (ihl - 5) * 4],
    }
    @payload payload: [u8],
}

struct Ipv6 {
    @check(version == 6) version: b<4>,
    tc_hi: b<4>,
    tc_lo: b<4>,
    flow_hi: b<4>,
    flow_lo: u16,
    payload_len: u16,
    @discriminator next_header: u8,
    hop_limit: u8,
    src: [u8; 16],
    dst: [u8; 16],
    @payload payload: [u8; payload_len],
}
"#;

const SCTP_DSL: &str = r#"
struct SctpChunk {
    chunk_type: u8,
    flags: u8,
    @range(4, 65535) length: u16,
    @len(length - 4) @greedy(unsafe_eof) value: [u8],
}

struct Sctp {
    src_port: u16,
    dst_port: u16,
    vtag: u32,
    checksum: u32,
    @greedy(unsafe_eof) @max_iter(256) chunks: [SctpChunk],
}
"#;

const DHCP_DSL: &str = r#"
struct DhcpOption {
    code: u8,
    body: union(code) {
        0 => Pad { },
        255 => End { },
        _ => Generic { len: u8, data: [u8; len] },
    },
}

struct Dhcp {
    op: u8,
    htype: u8,
    hlen: u8,
    hops: u8,
    xid: u32,
    secs: u16,
    flags: u16,
    ciaddr: [u8; 4],
    yiaddr: [u8; 4],
    siaddr: [u8; 4],
    giaddr: [u8; 4],
    chaddr: [u8; 16],
    sname: [u8; 64],
    file: [u8; 128],
    magic = x63825363,
    @greedy(unsafe_eof) @max_iter(256) options: [DhcpOption],
}
"#;

#[test]
fn ethernet_writer_round_trips_and_pins_wire_bytes() {
    let test_body = r#"
        let content = EthernetIIContent {
            dst: [0x00, 0x1b, 0x21, 0x3c, 0x9d, 0xf8],
            src: [0x00, 0x19, 0x06, 0xea, 0xb8, 0xc1],
            ethertype: 0x0800,
            payload: &[0xde, 0xad, 0xbe, 0xef],
        };
        let bytes = EthernetIIWriter::to_vec(&content);
        let expected: Vec<u8> = vec![
            0x00, 0x1b, 0x21, 0x3c, 0x9d, 0xf8,
            0x00, 0x19, 0x06, 0xea, 0xb8, 0xc1,
            0x08, 0x00,
            0xde, 0xad, 0xbe, 0xef,
        ];
        assert_eq!(bytes, expected);

        let lens = EthernetIILens { payload: content.payload.len() };
        assert_eq!(EthernetIIWriter::encoded_len(&lens), 18);

        let (eth, rem) = EthernetII::parse(&bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            eth.dst().unwrap().collect::<Result<Vec<u8>, _>>().unwrap(),
            vec![0x00, 0x1b, 0x21, 0x3c, 0x9d, 0xf8]
        );
        assert_eq!(
            eth.src().unwrap().collect::<Result<Vec<u8>, _>>().unwrap(),
            vec![0x00, 0x19, 0x06, 0xea, 0xb8, 0xc1]
        );
        assert_eq!(eth.ethertype(), 0x0800);
        assert_eq!(
            eth.payload().unwrap().collect::<Result<Vec<u8>, _>>().unwrap(),
            vec![0xde, 0xad, 0xbe, 0xef]
        );

        let mut buf = vec![0u8; EthernetIIWriter::encoded_len(&lens)];
        let mut w = EthernetIIWriter::new(&mut buf, lens).unwrap();
        w.dst_mut().copy_from_slice(&content.dst);
        w.src_mut().copy_from_slice(&content.src);
        w.set_ethertype(content.ethertype);
        w.payload_mut().copy_from_slice(content.payload);
        assert_eq!(buf, expected);

        assert!(matches!(
            EthernetIIWriter::new(&mut [0u8; 10], EthernetIILens { payload: 4 }),
            Err(binparse::WriteError::NotEnoughSpace { .. })
        ));
    "#;
    run_round_trip("ethernet", ETHERNET_DSL, test_body);
}

#[test]
fn vlan_writer_round_trips_and_derives_constant() {
    let test_body = r#"
        let content = VlanContent {
            dst: [1, 2, 3, 4, 5, 6],
            src: [10, 11, 12, 13, 14, 15],
            pcp: 5,
            dei: 1,
            vid_hi: 0xa,
            vid_lo: 0xbc,
            ethertype: 0x0800,
            payload: &[0xde, 0xad, 0xbe, 0xef],
        };
        let bytes = VlanWriter::to_vec(&content);
        assert_eq!(&bytes[12..14], &[0x81, 0x00]);

        let (vlan, rem) = Vlan::parse(&bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            vlan.dst().unwrap().collect::<Result<Vec<u8>, _>>().unwrap(),
            vec![1, 2, 3, 4, 5, 6]
        );
        assert_eq!(
            vlan.src().unwrap().collect::<Result<Vec<u8>, _>>().unwrap(),
            vec![10, 11, 12, 13, 14, 15]
        );
        assert_eq!(vlan.tpid(), 0x8100);
        assert_eq!(vlan.pcp(), 5);
        assert_eq!(vlan.dei(), 1);
        assert_eq!(vlan.vid_hi(), 0xa);
        assert_eq!(vlan.vid_lo(), 0xbc);
        assert_eq!(vlan.ethertype(), 0x0800);
        assert_eq!(
            vlan.payload().unwrap().collect::<Result<Vec<u8>, _>>().unwrap(),
            vec![0xde, 0xad, 0xbe, 0xef]
        );
    "#;
    run_round_trip("vlan", VLAN_DSL, test_body);
}

#[test]
fn tcp_option_writer_round_trips_and_pins_wire_bytes() {
    // NOTE: the Tcp struct itself produces NO writer (the classifier rejects its
    // `@len(...) options: TcpOptionList` struct-ref field plus trailing greedy
    // payload), and the TcpOption `Generic` variant produces no writer (its
    // `data: [u8; len - 2]` array depends on the derived `len` field). Only the
    // discriminant-only `Eol`/`Nop` TcpOption variants are writeable, so the wire
    // bytes pinned here are for TcpOption, not the full Tcp header.
    let test_body = r#"
        let nop = TcpOptionContent { body: TcpOptionBodyContent::Nop(NopContent {}) };
        let bytes = TcpOptionWriter::to_vec(&nop);
        assert_eq!(bytes, vec![0x01]);
        let (opt, rem) = TcpOption::parse(&bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(opt.kind(), 1);
        assert!(matches!(opt.body().unwrap(), TcpOption_body::Nop(_)));

        let eol = TcpOptionContent { body: TcpOptionBodyContent::Eol(EolContent {}) };
        let bytes = TcpOptionWriter::to_vec(&eol);
        assert_eq!(bytes, vec![0x00]);
        let (opt, rem) = TcpOption::parse(&bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(opt.kind(), 0);
        assert!(matches!(opt.body().unwrap(), TcpOption_body::Eol(_)));
    "#;
    run_round_trip("tcp-option", TCP_DSL, test_body);
}

#[test]
fn ipv6_writer_round_trips_and_derives_payload_len() {
    // NOTE: only Ipv6 from ip.bp produces a writer; Ipv4 produces none because of
    // its `if (ihl > 5) { options }` conditional block, which the writer classifier
    // does not support.
    let test_body = r#"
        let payload: Vec<u8> = vec![
            0xc3, 0x50, 0x00, 0x35, 0x00, 0x0c, 0x1a, 0x2b, 0xde, 0xad, 0xbe, 0xef,
        ];
        let content = Ipv6Content {
            version: 6,
            tc_hi: 0,
            tc_lo: 0,
            flow_hi: 0,
            flow_lo: 0,
            next_header: 17,
            hop_limit: 64,
            src: [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01],
            dst: [0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x02],
            payload: &payload,
        };
        let lens = Ipv6Lens { payload: payload.len() };
        assert_eq!(Ipv6Writer::encoded_len(&lens), 40 + payload.len());

        let bytes = Ipv6Writer::to_vec(&content);
        assert_eq!(bytes[0] >> 4, 6);
        assert_eq!(&bytes[4..6], &[0x00, 0x0c]);

        let (ip, rem) = Ipv6::parse(&bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(ip.version(), 6);
        assert_eq!(ip.payload_len(), 12);
        assert_eq!(ip.next_header(), 17);
        assert_eq!(ip.hop_limit(), 64);
        assert_eq!(
            ip.src().unwrap().collect::<Result<Vec<u8>, _>>().unwrap(),
            vec![0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01]
        );
        assert_eq!(
            ip.dst().unwrap().collect::<Result<Vec<u8>, _>>().unwrap()[15],
            0x02
        );
        assert_eq!(
            ip.payload().unwrap().collect::<Result<Vec<u8>, _>>().unwrap(),
            payload
        );

        let mut big = vec![0u8; 70040];
        assert!(matches!(
            Ipv6Writer::new(&mut big, Ipv6Lens { payload: 70000 }),
            Err(binparse::WriteError::ValueTooLarge { .. })
        ));
        assert!(matches!(
            Ipv6Writer::new(&mut [0u8; 10], Ipv6Lens { payload: 4 }),
            Err(binparse::WriteError::NotEnoughSpace { .. })
        ));
    "#;
    run_round_trip("ipv6", IP_DSL, test_body);
}

#[test]
fn sctp_chunk_writer_round_trips() {
    // NOTE: only SctpChunk from sctp.bp produces a writer; Sctp produces none
    // because its `chunks: [SctpChunk]` array-of-structs tail is not writeable.
    // SctpChunk's `length` is a plain Content field (not auto-derived), so the
    // round-trip sets length = value.len() + 4 to satisfy the reader's
    // `@len(length - 4)` slicing of `value`.
    let test_body = r#"
        let value: Vec<u8> = vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let content = SctpChunkContent {
            chunk_type: 0x01,
            flags: 0x00,
            length: (value.len() + 4) as u16,
            value: &value,
        };
        let lens = SctpChunkLens { value: value.len() };
        assert_eq!(SctpChunkWriter::encoded_len(&lens), 4 + value.len());

        let bytes = SctpChunkWriter::to_vec(&content);
        assert_eq!(&bytes[0..4], &[0x01, 0x00, 0x00, 0x0a]);

        let (chunk, rem) = SctpChunk::parse(&bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(chunk.chunk_type(), 0x01);
        assert_eq!(chunk.flags(), 0x00);
        assert_eq!(chunk.length(), 10);
        assert_eq!(
            chunk.value().unwrap().collect::<Result<Vec<u8>, _>>().unwrap(),
            value
        );
    "#;
    run_round_trip("sctp-chunk", SCTP_DSL, test_body);
}

#[test]
fn dhcp_option_writer_round_trips() {
    // NOTE: only DhcpOption from dhcp.bp produces a writer; Dhcp produces none
    // because its `options: [DhcpOption]` array-of-structs tail is not writeable.
    // The DhcpOption `Generic` variant produces no writer (its `data: [u8; len]`
    // array depends on the derived `len` field), so only the discriminant-only
    // `Pad`/`End` variants are exercised here.
    let test_body = r#"
        let pad = DhcpOptionContent { body: DhcpOptionBodyContent::Pad(PadContent {}) };
        let bytes = DhcpOptionWriter::to_vec(&pad);
        assert_eq!(bytes, vec![0x00]);
        let (opt, rem) = DhcpOption::parse(&bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(opt.code(), 0);
        assert!(matches!(opt.body().unwrap(), DhcpOption_body::Pad(_)));

        let end = DhcpOptionContent { body: DhcpOptionBodyContent::End(EndContent {}) };
        let bytes = DhcpOptionWriter::to_vec(&end);
        assert_eq!(bytes, vec![0xff]);
        let (opt, rem) = DhcpOption::parse(&bytes).unwrap();
        assert!(rem.is_empty());
        assert_eq!(opt.code(), 255);
        assert!(matches!(opt.body().unwrap(), DhcpOption_body::End(_)));
    "#;
    run_round_trip("dhcp-option", DHCP_DSL, test_body);
}
