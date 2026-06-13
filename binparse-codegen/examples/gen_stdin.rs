use std::io::Read;

fn main() {
    let mut dsl = String::new();
    std::io::stdin().read_to_string(&mut dsl).unwrap();
    let ast = binparse_dsl_parse::parse_str(&dsl)
        .inspect_err(|e| eprintln!("{e}"))
        .unwrap();
    let code = binparse_codegen::CodeGen::generate(&ast)
        .inspect_err(|e| eprintln!("{e}"))
        .unwrap();
    println!("{code}");
}
