use binparse_codegen::CodeGen;
use binparse_dsl as ast;

#[test]
fn test_error_enum_generation() {
    let error_def = ast::Definition::Error(vec![
        ast::ErrorVariant {
            name: "MyError",
            fields: vec![],
        },
        ast::ErrorVariant {
            name: "DetailedError",
            fields: vec![("code", ast::Primitive::U8), ("val", ast::Primitive::U16)],
        },
    ]);

    let output = CodeGen::generate(&[error_def]).expect("generation failed");

    // Check for enum definition
    assert!(output.contains("pub enum Error"));
    assert!(output.contains("UnexpectedEof"));
    assert!(output.contains("MyError"));
    assert!(output.contains("DetailedError"));
    assert!(output.contains("code: u8"));
    assert!(output.contains("val: u16"));
    assert!(output.contains("derive(Debug, Clone, PartialEq)"));
}
