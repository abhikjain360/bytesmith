use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn generated_code(dsl: &str) -> String {
    let ast = binparse_dsl_parse::parse_str(dsl).expect("failed to parse DSL");
    binparse_codegen::CodeGen::generate(&ast).expect("failed to generate code")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("binparse-codegen should have a workspace parent")
        .to_path_buf()
}

fn write_runtime_crate(code: &str) -> PathBuf {
    let root = workspace_root();
    let test_dir = root
        .join("target")
        .join("generated-runtime-tests")
        .join(format!("runtime-{}", std::process::id()));

    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(test_dir.join("src")).expect("failed to create runtime test crate");

    let binparse_path = root.join("binparse");
    fs::write(
        test_dir.join("Cargo.toml"),
        format!(
            r#"[package]
name = "generated-runtime-test"
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
fn double_it(value: u16) -> u32 {{
    u32::from(value) * 2
}}

fn parse_cstring(data: &[u8]) -> (String, usize) {{
    binparse::hooks::cstring(data)
}}

{code}

#[cfg(test)]
mod tests {{
    use super::*;

    fn assert_parse_no_panic<F>(name: &str, data: &[u8], parse: F)
    where
        F: Fn(&[u8]),
    {{
        for len in 0..=data.len() {{
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {{
                parse(&data[..len]);
            }}));
            assert!(result.is_ok(), "{{name}} panicked at len {{len}}");
        }}
    }}

    #[test]
    fn baseline_valid_packet_decodes() {{
        let data = [
            1, 0x34, 0x12, 0x01, 0x02, 0x03, 0x04, 0b1010_1101, 9, 8, 7, 0xaa, 0x01, 0x02,
            0x78, 0x56, 0x55, 0xcd, 0xab, 0xfe,
        ];
        let (packet, rem) = Baseline::parse(&data).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.n(), 1);
        assert_eq!(packet.word(), 0x1234);
        assert_eq!(packet.be(), 0x0102_0304);
        assert_eq!(packet.flag_a(), 5);
        assert_eq!(packet.flag_b(), 13);

        let fixed = packet
            .fixed()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(fixed, vec![9, 8, 7]);

        let inner = packet.inner().unwrap();
        assert_eq!(inner.a(), 0xaa);
        assert_eq!(inner.b(), 0x0102);

        let dyns = packet
            .dyns()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(dyns, vec![0x5678]);
        assert_eq!(packet.pair(), (0x55, 0xabcd));

        match packet.payload() {{
            Baseline_payload::One(one) => assert_eq!(one.x(), 0xfe),
            Baseline_payload::Unknown(_) => panic!("expected One payload"),
        }}
    }}

    #[test]
    fn offsets_report_absolute_bit_ranges() {{
        let data = [
            1, 0x34, 0x12, 0x01, 0x02, 0x03, 0x04, 0b1010_1101, 9, 8, 7, 0xaa, 0x01, 0x02,
            0x78, 0x56, 0x55, 0xcd, 0xab, 0xfe,
        ];
        let (packet, _) = Baseline::parse(&data).unwrap();
        assert_eq!(packet.n_start_offset(), binparse::Len::ZERO);
        assert_eq!(packet.n_bit_range(), 0..8);
        assert_eq!(packet.word_bit_range(), 8..24);
        assert_eq!(packet.be_bit_range(), 24..56);
        assert_eq!(packet.flag_a_bit_range(), 56..59);
        assert_eq!(packet.flag_b_bit_range(), 59..64);
        assert_eq!(packet.fixed_bit_range(), 64..88);
        assert_eq!(packet.inner_bit_range(), 88..112);
        assert_eq!(packet.dyns_bit_range(), 112..128);
        assert_eq!(packet.pair_bit_range(), 128..152);
        assert_eq!(packet.payload_bit_range(), 152..160);
        assert_eq!(packet.payload_end_offset(), binparse::Len {{ byte: 20, bit: 0 }});

        let inner = packet.inner().unwrap();
        assert_eq!(inner.a_bit_range(), 0..8);
        assert_eq!(inner.b_bit_range(), 8..24);
        let inner_base = packet.inner_start_offset().bits();
        assert_eq!(inner_base + inner.b_bit_range().start, 96);
        assert_eq!(inner_base + inner.b_bit_range().end, 112);
    }}

    #[test]
    fn cross_byte_bitfield_offsets_and_values() {{
        let data = [0xad, 0xad];
        let (packet, rem) = CrossByte::parse(&data).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.high(), 21);
        assert_eq!(packet.mid(), 45);
        assert_eq!(packet.low(), 13);
        assert_eq!(packet.high_bit_range(), 0..5);
        assert_eq!(packet.mid_bit_range(), 5..11);
        assert_eq!(packet.low_bit_range(), 11..16);
        assert_eq!(packet.mid_start_offset(), binparse::Len {{ byte: 0, bit: 5 }});
        assert_eq!(packet.mid_end_offset(), binparse::Len {{ byte: 1, bit: 3 }});
        assert_parse_no_panic("CrossByte", &data, |data| {{
            let _ = CrossByte::parse(data);
        }});
    }}

    #[test]
    fn size_expression_valid_packet_decodes() {{
        let data = [0, 0, 0, 0, 0, 0, 0, 2, 1, 2, 3, 4];
        let (packet, rem) = SizeExpr::parse(&data).unwrap();
        assert!(rem.is_empty());
        let xs = packet
            .xs()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(xs, vec![1, 2, 3, 4]);
        assert_eq!(packet.xs_bit_range(), 64..96);
    }}

    #[test]
    fn size_expression_overflow_saturates_instead_of_panicking() {{
        let data = [0xff; 8];
        let err = SizeExpr::parse(&data).map(|_| ()).unwrap_err();
        assert_eq!(
            err,
            binparse::ParseError::NotEnoughData {{
                expected: usize::MAX,
                got: 8,
            }}
        );
        assert_parse_no_panic("SizeExpr", &data, |data| {{
            let _ = SizeExpr::parse(data);
        }});
    }}

    #[test]
    fn huge_array_count_errors_instead_of_overflowing() {{
        let data = [0xff; 8];
        let err = Huge::parse(&data).map(|_| ()).unwrap_err();
        assert_eq!(
            err,
            binparse::ParseError::NotEnoughData {{
                expected: usize::MAX,
                got: 8,
            }}
        );
        assert_parse_no_panic("Huge", &data, |data| {{
            let _ = Huge::parse(data);
        }});
    }}

    #[test]
    fn baseline_parse_does_not_panic_on_truncation() {{
        let data = [
            1, 0x34, 0x12, 0x01, 0x02, 0x03, 0x04, 0b1010_1101, 9, 8, 7, 0xaa, 0x01, 0x02,
            0x78, 0x56, 0x55, 0xcd, 0xab, 0xfe,
        ];
        assert_parse_no_panic("Baseline", &data, |data| {{
            let _ = Baseline::parse(data);
        }});
    }}

    #[test]
    fn hooks_decode_and_do_not_panic_on_truncation() {{
        let data = [3, 0, 2, b'h', b'i', 0];
        let (packet, rem) = Hooked::parse(&data).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.prefix(), 3);
        assert_eq!(packet.value(), 4);
        assert_eq!(packet.name(), "hi");
        assert_eq!(packet.value_bit_range(), 8..24);
        assert_eq!(packet.name_bit_range(), 24..48);
        assert_parse_no_panic("Hooked", &data, |data| {{
            let _ = Hooked::parse(data);
        }});
    }}

    #[test]
    fn struct_array_decodes_and_does_not_panic_on_truncation() {{
        let data = [2, 1, 0x02, 0x03, 4, 0x05, 0x06];
        let (packet, rem) = StructArray::parse(&data).unwrap();
        assert!(rem.is_empty());
        let items = packet
            .items()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].a(), 1);
        assert_eq!(items[0].b(), 0x0203);
        assert_eq!(items[1].a(), 4);
        assert_eq!(items[1].b(), 0x0506);
        assert_eq!(packet.items_bit_range(), 8..56);
        assert_parse_no_panic("StructArray", &data, |data| {{
            let _ = StructArray::parse(data);
        }});

        let short = [10, 1, 0x02];
        assert_parse_no_panic("StructArray short", &short, |data| {{
            let _ = StructArray::parse(data);
        }});
    }}

    #[test]
    fn signed_integers_decode_with_endian_inheritance() {{
        let mut data = vec![0xff, 0xfe, 0xff];
        data.extend((-3i32).to_be_bytes());
        data.extend((-4i64).to_le_bytes());
        data.extend((-5i128).to_le_bytes());
        data.extend(5i16.to_le_bytes());
        data.extend((-5i16).to_le_bytes());
        data.extend([0x7f, 0x80]);
        let (packet, rem) = Signed::parse(&data).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.a(), -1i8);
        assert_eq!(packet.b(), -2i16);
        assert_eq!(packet.c(), -3i32);
        assert_eq!(packet.d(), -4i64);
        assert_eq!(packet.e(), -5i128);
        let vals = packet
            .vals()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(vals, vec![5i16, -5i16]);
        let small = packet
            .small()
            .unwrap()
            .collect::<binparse::ParseResult<Vec<_>>>()
            .unwrap();
        assert_eq!(small, vec![127i8, -128i8]);
        assert_eq!(packet.a_bit_range(), 0..8);
        assert_eq!(packet.b_bit_range(), 8..24);
        assert_parse_no_panic("Signed", &data, |data| {{
            let _ = Signed::parse(data);
        }});
    }}

    #[test]
    fn ipv4_version_and_ihl_decode_msb_first() {{
        let data = [0x45];
        let (packet, rem) = Ipv4Start::parse(&data).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.version(), 4);
        assert_eq!(packet.ihl(), 5);
        assert_eq!(packet.version_bit_range(), 0..4);
        assert_eq!(packet.ihl_bit_range(), 4..8);
        assert_parse_no_panic("Ipv4Start", &data, |data| {{
            let _ = Ipv4Start::parse(data);
        }});
    }}

    #[test]
    fn tcp_flags_decode_without_hooks() {{
        let data = [0x50, 0b0001_1000];
        let (packet, rem) = TcpFlags::parse(&data).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.data_offset(), 5);
        assert_eq!(packet.reserved(), 0);
        assert_eq!(packet.ns(), 0);
        assert_eq!(packet.cwr(), 0);
        assert_eq!(packet.ece(), 0);
        assert_eq!(packet.urg(), 0);
        assert_eq!(packet.ack(), 1);
        assert_eq!(packet.psh(), 1);
        assert_eq!(packet.rst(), 0);
        assert_eq!(packet.syn(), 0);
        assert_eq!(packet.fin(), 0);
        assert_eq!(packet.ack_bit_range(), 11..12);
        assert_parse_no_panic("TcpFlags", &data, |data| {{
            let _ = TcpFlags::parse(data);
        }});
    }}

    #[test]
    fn validated_packet_decodes() {{
        let data = [0x89, 0x50, 0x4e, 0x47, 0x45, 0x00, 0x14, 0b00_000011];
        let (packet, rem) = Validated::parse(&data).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.magic(), 0x89504e47);
        assert_eq!(packet.version(), 4);
        assert_eq!(packet.ihl(), 5);
        assert_eq!(packet.total_len(), 20);
        assert_eq!(packet.reserved(), 0);
        assert_eq!(packet.flags(), 3);
        assert_parse_no_panic("Validated", &data, |data| {{
            let _ = Validated::parse(data);
        }});
    }}

    #[test]
    fn validation_failures_report_field_and_actual_value() {{
        let valid = [0x89, 0x50, 0x4e, 0x47, 0x45, 0x00, 0x14, 0b00_000011];

        let mut bad_magic = valid;
        bad_magic[0] = 0x88;
        assert_eq!(
            Validated::parse(&bad_magic).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Validated.magic",
                actual: 0x88504e47,
            }}
        );

        let mut bad_version = valid;
        bad_version[4] = 0x55;
        assert_eq!(
            Validated::parse(&bad_version).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Validated.version",
                actual: 5,
            }}
        );

        let mut bad_len = valid;
        bad_len[6] = 0x13;
        assert_eq!(
            Validated::parse(&bad_len).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Validated.total_len",
                actual: 19,
            }}
        );

        let mut bad_reserved = valid;
        bad_reserved[7] = 0b01_000011;
        assert_eq!(
            Validated::parse(&bad_reserved).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Validated.reserved",
                actual: 1,
            }}
        );

        let mut bad_flags = valid;
        bad_flags[7] = 0b00_000111;
        assert_eq!(
            Validated::parse(&bad_flags).map(|_| ()).unwrap_err(),
            binparse::ParseError::ValidationFailed {{
                field: "Validated.flags",
                actual: 7,
            }}
        );
    }}

    #[test]
    fn truncation_is_reported_before_validation() {{
        let bad_magic = [0x88, 0x50, 0x4e];
        assert_eq!(
            Validated::parse(&bad_magic).map(|_| ()).unwrap_err(),
            binparse::ParseError::NotEnoughData {{
                expected: 4,
                got: 3,
            }}
        );
    }}

    #[test]
    fn lsb_bit_order_decodes_with_field_override() {{
        let data = [0b1010_1101, 0b0100_0011];
        let (packet, rem) = LsbFlags::parse(&data).unwrap();
        assert!(rem.is_empty());
        assert_eq!(packet.low(), 5);
        assert_eq!(packet.high(), 21);
        assert_eq!(packet.top(), 4);
        assert_eq!(packet.bottom(), 3);
        assert_parse_no_panic("LsbFlags", &data, |data| {{
            let _ = LsbFlags::parse(data);
        }});
    }}
}}
"#
        ),
    )
    .expect("failed to write runtime lib.rs");

    test_dir
}

#[test]
fn generated_code_compiles_and_handles_runtime_baseline() {
    let dsl = r#"
struct Inner {
    a: u8,
    b: u16,
}

@endian(little)
struct Baseline {
    n: u8,
    word: u16,
    @endian(big) be: u32,
    flag_a: b<3>,
    flag_b: b<5>,
    fixed: [u8; 3],
    inner: Inner,
    dyns: [u16; n],
    pair: concat(u8, u16),
    payload: union(n) {
        1 => One { x: u8 },
        _ => Unknown { },
    },
}

struct Hooked {
    prefix: u8,
    @hook(double_it, u32)
    value: u16,
    @hook(parse_cstring, String)
    name: [u8],
}

struct StructArray {
    count: u8,
    items: [Inner; count],
}

struct CrossByte {
    high: b<5>,
    mid: b<6>,
    low: b<5>,
}

struct Huge {
    n: u64,
    xs: [u128; n],
}

struct SizeExpr {
    n: u64,
    xs: [u8; n * 2],
}

@endian(little)
struct Signed {
    a: i8,
    b: i16,
    @endian(big) c: i32,
    d: i64,
    e: i128,
    vals: [i16; 2],
    small: [i8; 2],
}

struct Ipv4Start {
    version: b<4>,
    ihl: b<4>,
}

struct TcpFlags {
    data_offset: b<4>,
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
}

@bit_order(lsb)
struct LsbFlags {
    low: b<3>,
    high: b<5>,
    @bit_order(msb) top: b<4>,
    @bit_order(msb) bottom: b<4>,
}

struct Validated {
    magic = x89504e47,
    @check(version == 4) version: b<4>,
    ihl: b<4>,
    @range(20, 60) total_len: u16,
    reserved = b00,
    @check(flags <= 3) flags: b<6>,
}
"#;

    let code = generated_code(dsl);
    let test_dir = write_runtime_crate(&code);
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
