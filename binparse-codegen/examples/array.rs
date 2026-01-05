const DSL_STRING: &'static str = r#"
struct Item {
    val: u8
}
struct Container {
    count: u8,
    fixed_items: [Item; 2],
    dyn_items: [Item; count],
    raw_bytes: [u8; 4]
}
"#;

fn main() {
    let ast = binparse_dsl_parse::parse_str(DSL_STRING).unwrap();
    let code = binparse_codegen::CodeGen::generate(&ast).unwrap();
    println!("{code}");
}
