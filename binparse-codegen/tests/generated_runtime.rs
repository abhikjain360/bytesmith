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
        assert_eq!(packet.flag_b(), 21);

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
        assert_eq!(packet.high(), 13);
        assert_eq!(packet.mid(), 45);
        assert_eq!(packet.low(), 21);
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
