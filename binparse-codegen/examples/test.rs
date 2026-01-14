const DSL_STRING: &str = r#"
struct IcmpPacket {
    icmp_type: u8,
    code: u8,
    checksum: u16,
    payload: union(icmp_type) {
        0 | 8 => Echo { id: u16, seq: u16 },
        3 => DestUnreachable { unused: u32 },
        11 => TimeExceeded { unused: u32 },
        _ => Unknown { },
    },
}
"#;

fn main() {
    let ast = binparse_dsl_parse::parse_str(DSL_STRING)
        .inspect_err(|e| eprintln!("{e}"))
        .unwrap();
    let code = binparse_codegen::CodeGen::generate(&ast)
        .inspect_err(|e| eprintln!("{e}"))
        .unwrap();
    println!("{code}");
}
