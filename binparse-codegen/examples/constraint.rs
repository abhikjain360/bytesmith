const DSL_STRING: &'static str = r#"struct Header {
    magic = xCAFEBABE,
    version: u8
}
"#;

fn main() {
    let ast = binparse_dsl_parse::parse_str(DSL_STRING).unwrap();
    let code = binparse_codegen::CodeGen::generate(&ast).unwrap();
    println!("{code}");
}
