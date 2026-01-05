const DSL_STRING: &'static str = r#"
struct Flags {
    f1: b<1>,
    reserved: b<7>,
    if (f1 == 1) {
        val: u8
    }
}
"#;

fn main() {
    let ast = binparse_dsl_parse::parse_str(DSL_STRING).unwrap();
    let code = binparse_codegen::CodeGen::generate(&ast).unwrap();
    println!("{code}");
}
