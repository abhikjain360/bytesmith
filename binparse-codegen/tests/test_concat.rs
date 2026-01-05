use binparse_codegen::CodeGen;

#[test]
fn test_concat_generation() {
    let input = r#"
        struct ConcatTest {
            val: concat(
                a: b<4>,
                b: b<4>
            )
        }
    "#;

    let ast = binparse_dsl_parse::parse_str(input).expect("failed to parse DSL");
    let code = CodeGen::generate(&ast).expect("failed to generate code");

    // Check return type
    assert!(code.contains("let mut val: u8 = 0;"));

    // Check first shift (a: b<4>)
    // val = (val << 4) | ((self.data[offset + 0] >> 4) & 15) as u8;
    assert!(code.contains("val << 4"));
    assert!(code.contains("self.data[offset + 0] >> 4"));
    assert!(code.contains("& 15"));

    // Check second shift (b: b<4>)
    assert!(code.contains("self.data[offset + 0] >> 0"));
}

#[test]
fn test_concat_disjoint() {
    let input = r#"
        struct ConcatDisjoint {
            prefix: b<4>,
            val: concat(
                x: b<4>,
                y: b<8>
            )
        }
    "#;

    let ast = binparse_dsl_parse::parse_str(input).expect("failed to parse DSL");
    let code = CodeGen::generate(&ast).expect("failed to generate code");

    // prefix takes 0..4
    // val takes 4..16 (12 bits -> u16)

    assert!(code.contains("let mut val: u16 = 0;"));

    // x: b<4> at offset 4
    // byte 0, bit 4. width 4.
    // shift = 8 - 4 - 4 = 0. mask = 15.
    assert!(code.contains("self.data[offset + 0] >> 0"));

    // y: b<8> at offset 8 (byte 1, bit 0)
    // byte 1, bit 0. width 8.
    // shift = 0. mask = 255.
    assert!(code.contains("self.data[offset + 1] >> 0"));
    assert!(code.contains("& 255"));
}
