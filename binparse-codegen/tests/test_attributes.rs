use binparse_codegen::CodeGen;
use binparse_dsl as ast;

fn generate_and_check(src: &[ast::Definition], expected_snippets: &[&str]) {
    let output = CodeGen::generate(src).expect("generation failed");
    for snippet in expected_snippets {
        assert!(
            output.contains(snippet),
            "Output missing snippet: {}\nFull output:\n{}",
            snippet,
            output
        );
    }
}

#[test]
fn test_field_len_struct() {
    // struct Inner { a: u8 }
    // struct Outer { inner: Inner @len(10) }
    // Should consume 10 bytes for inner.

    let inner = ast::Struct {
        name: "Inner",
        attributes: vec![],
        items: vec![ast::StructItem::Field(ast::Field {
            name: "a",
            attributes: vec![],
            value: ast::FieldValue::Type(ast::Type::Primitive(ast::Primitive::U8)),
        })],
    };

    let outer = ast::Struct {
        name: "Outer",
        attributes: vec![],
        items: vec![ast::StructItem::Field(ast::Field {
            name: "inner",
            attributes: vec![ast::Attribute {
                name: "len",
                args: vec![ast::AttributeArg::Math(ast::MathExpr::Atom(
                    ast::NumericAtom::Literal(ast::NumericLiteral::Decimal(10)),
                ))],
            }],
            value: ast::FieldValue::Type(ast::Type::StructRef(vec!["Inner"])),
        })],
    };

    generate_and_check(
        &[
            ast::Definition::Struct(inner),
            ast::Definition::Struct(outer),
        ],
        &["let limit = 10 as usize;"],
    );
}

#[test]
fn test_field_len_array_greedy() {
    // struct Outer { arr: [u8] @len(5) }
    // Should parse u8s until 5 bytes consumed.

    let outer = ast::Struct {
        name: "Outer",
        attributes: vec![],
        items: vec![ast::StructItem::Field(ast::Field {
            name: "arr",
            attributes: vec![ast::Attribute {
                name: "len",
                args: vec![ast::AttributeArg::Math(ast::MathExpr::Atom(
                    ast::NumericAtom::Literal(ast::NumericLiteral::Decimal(5)),
                ))],
            }],
            value: ast::FieldValue::Type(ast::Type::Array(Box::new(ast::ArrayType {
                elem_ty: ast::Type::Primitive(ast::Primitive::U8),
                size_expr: None, // Greedy
            }))),
        })],
    };

    generate_and_check(
        &[ast::Definition::Struct(outer)],
        &["let limit = 5 as usize;", "let count = limit / 1;"],
    );
}

#[test]
fn test_field_len_array_counted() {
    // struct Outer { arr: [u8; 5] @len(10) }
    // Should ensure 5 items found within 10 bytes.

    let outer = ast::Struct {
        name: "Outer",
        attributes: vec![],
        items: vec![ast::StructItem::Field(ast::Field {
            name: "arr",
            attributes: vec![ast::Attribute {
                name: "len",
                args: vec![ast::AttributeArg::Math(ast::MathExpr::Atom(
                    ast::NumericAtom::Literal(ast::NumericLiteral::Decimal(10)),
                ))],
            }],
            value: ast::FieldValue::Type(ast::Type::Array(Box::new(ast::ArrayType {
                elem_ty: ast::Type::Primitive(ast::Primitive::U8),
                size_expr: Some(ast::MathExpr::Atom(ast::NumericAtom::Literal(
                    ast::NumericLiteral::Decimal(5),
                ))),
            }))),
        })],
    };

    generate_and_check(
        &[ast::Definition::Struct(outer)],
        &["let limit = 10 as usize;", "if size > limit {"],
    );
}
