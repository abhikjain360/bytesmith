use binparse_codegen::CodeGen;

#[test]
fn test_union_simple() {
    let input = r#"
        struct Packet {
            t: u8,
            body: union(t) {
                0 | 1 => VariantA {
                    x: u8
                },
                2 => VariantB {
                    y: u16
                }
            }
        }
    "#;

    let ast = binparse_dsl_parse::parse_str(input).expect("failed to parse DSL");
    let code = CodeGen::generate(&ast).expect("failed to generate code");

    // Check enum definition
    assert!(code.contains("pub enum Packet_body<'a> {"));
    assert!(code.contains("VariantA(VariantA<'a>),"));
    assert!(code.contains("VariantB(VariantB<'a>),"));

    // Check struct definitions
    assert!(code.contains("pub struct VariantA<'a> {"));
    assert!(code.contains("pub struct VariantB<'a> {"));

    // Check accessor
    // We generated Result<..., binparse::Error> in struct_.rs
    // Note: binparse-codegen uses Ident::new_raw for fields, so they are prefixed with r#
    assert!(code.contains(
        "pub fn r#body(&self) -> Result<(Packet_body<'a>, usize, usize), binparse::Error> {"
    ));
    assert!(code.contains("match self.r#t().0 {"));

    assert!(code.contains("0 | 1 => {"));
    assert!(code.contains("VariantA::parse(slice)?"));
    assert!(code.contains("Packet_body::VariantA"));
}

#[test]
fn test_union_tuple() {
    let input = r#"
        struct TuplePacket {
            maj: u8,
            min: u8,
            body: union(maj, min) {
                (1, 0) => V1 { x: u8 },
                (2, _) => V2 { y: u8 }
            }
        }
    "#;

    let ast = binparse_dsl_parse::parse_str(input).expect("failed to parse DSL");
    let code = CodeGen::generate(&ast).expect("failed to generate code");
    println!("{}", code);

    // The generated code uses match (var1, var2)
    assert!(code.contains("match (self.r#maj().0, self.r#min().0) {"));
    assert!(code.contains("(1, 0) => {"));
    assert!(code.contains("(2, _) => {"));
}
