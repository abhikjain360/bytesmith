use binparse_codegen::CodeGen;

#[test]
fn test_bitfield_array_generation() {
    let input = r#"
        struct Nibbles {
            vals: [b<4>; 4]
        }
    "#;

    let ast = binparse_dsl_parse::parse_str(input).expect("failed to parse DSL");
    let code = CodeGen::generate(&ast).expect("failed to generate code");

    println!("{}", code);

    // Verify key parts of the generated code
    assert!(code.contains("pub fn r#vals"));
    assert!(code.contains("impl Iterator"));
    assert!(code.contains("let start_bit = 0"));
}

#[test]
fn test_unaligned_bitfield_array() {
    let input = r#"
        struct Unaligned {
            flag: b<1>,
            rest: [b<1>; 7]
        }
    "#;

    let ast = binparse_dsl_parse::parse_str(input).expect("failed to parse DSL");
    let code = CodeGen::generate(&ast).expect("failed to generate code");

    println!("{}", code);

    // Check that 'rest' starts at bit offset 1
    assert!(code.contains("let start_bit = 1"));
    assert!(code.contains("pub fn r#rest"));
}
