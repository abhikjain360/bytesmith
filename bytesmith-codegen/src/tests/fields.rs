use super::*;

fn constraint_expr<'a>(ast: &'a [bytesmith_dsl::Definition<'a>]) -> &'a bytesmith_dsl::Expr<'a> {
    let bytesmith_dsl::Definition::Struct(s) = &ast[0] else {
        panic!("expected struct");
    };
    let bytesmith_dsl::StructItem::Field(f) = &s.items[0] else {
        panic!("expected field");
    };
    let bytesmith_dsl::FieldValue::Constraint(e) = &f.value else {
        panic!("expected constraint");
    };
    e
}
fn numeric_done_fields() -> Vec<crate::struct_::DoneField> {
    ["n", "m"]
        .into_iter()
        .map(|name| crate::struct_::DoneField {
            name: name.to_string(),
            field_type: crate::struct_::DoneFieldType::Primitive,
            conditional: false,
        })
        .collect()
}
#[test]
fn lower_bool_expr() {
    let ast = bytesmith_dsl_parse::parse_str("struct Foo { c = n == 1 && m < 2 }").unwrap();
    let lowered = crate::expr::lower(
        constraint_expr(&ast),
        crate::expr::ExprType::Bool,
        &numeric_done_fields(),
    )
    .unwrap();
    let expected = quote::quote! {
        ((((self.n() as usize) == (1usize))) && (((self.m() as usize) < (2usize))))
    };
    assert_eq!(lowered.tokens.to_string(), expected.to_string());
}
#[test]
fn lower_bool_rejects_numeric_expr() {
    let ast = bytesmith_dsl_parse::parse_str("struct Foo { c = n + 1 }").unwrap();
    let err = crate::expr::lower(
        constraint_expr(&ast),
        crate::expr::ExprType::Bool,
        &numeric_done_fields(),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("expression '(n + 1)' is a number but a boolean is required")
    );
}
#[test]
fn golden_constant_fields() {
    assert_generated_eq(
        r#"struct Magic { magic = xc0de, flags = b101 }"#,
        "constant_fields",
    );
}
#[test]
fn golden_check_and_range() {
    assert_generated_eq(
        r#"struct Checked { n: u8, @range(1, n + 1) @check(m >= n) m: u8 }"#,
        "check_and_range",
    );
}
#[test]
fn constant_decimal_infers_smallest_type() {
    let code = generate("struct Foo { small = 10, medium = 65536 }");
    assert!(code.contains("pub fn small(&mut self) -> u8"));
    assert!(code.contains("pub fn medium(&mut self) -> u32"));
}
#[test]
fn constant_hex_infers_type_from_width() {
    let code = generate("struct Foo { a = x0f, b = x0102030405060708 }");
    assert!(code.contains("pub fn a(&mut self) -> u8"));
    assert!(code.contains("pub fn b(&mut self) -> u64"));
}
#[test]
fn validate_attribute_is_an_alias_for_check() {
    let code = generate("struct Foo { @validate(n == 1) n: u8 }");
    assert!(code.contains("fn n_validate"));
    assert!(code.contains("self.n_validate()?"));
}
#[test]
fn constant_binary_too_wide_is_rejected() {
    let err = generate_err("struct Foo { f = b101010101 }");
    assert!(
        err.to_string()
            .contains("constant field literal width 9 is not supported")
    );
}
#[test]
fn constant_field_rejects_endian_on_single_byte() {
    let err = generate_err("struct Foo { @endian(little) f = x01 }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to single-byte integers")
    );
}
#[test]
fn check_on_struct_ref_is_rejected() {
    let err = generate_err("struct Inner { x: u8 } struct Foo { @check(1 == 1) inner: Inner }");
    assert!(
        err.to_string()
            .contains("@check and @range can only be applied to primitive and bitfield fields")
    );
}
#[test]
fn range_on_array_is_rejected() {
    let err = generate_err("struct Foo { @range(1, 2) xs: [u8; 4] }");
    assert!(
        err.to_string()
            .contains("@check and @range can only be applied to primitive and bitfield fields")
    );
}
#[test]
fn check_with_numeric_expr_is_rejected() {
    let err = generate_err("struct Foo { @check(n + 1) n: u8 }");
    assert!(
        err.to_string()
            .contains("is a number but a boolean is required")
    );
}
#[test]
fn range_with_bool_expr_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @range(n == 1, 5) m: u8 }");
    assert!(
        err.to_string()
            .contains("is a boolean but a number is required")
    );
}
#[test]
fn check_unknown_field_is_rejected() {
    let err = generate_err("struct Foo { @check(later == 1) n: u8, later: u8 }");
    assert!(
        err.to_string()
            .contains("references field 'later' which is unknown or not yet parsed")
    );
}
#[test]
fn check_wrong_arg_count_is_rejected() {
    let err = generate_err("struct Foo { @check(n == 1, n == 2) n: u8 }");
    assert!(
        err.to_string()
            .contains("@check requires exactly 1 argument(s), got 2")
    );
}
#[test]
fn range_wrong_arg_count_is_rejected() {
    let err = generate_err("struct Foo { @range(1) n: u8 }");
    assert!(
        err.to_string()
            .contains("@range requires exactly 2 argument(s), got 1")
    );
}
#[test]
fn range_min_greater_than_max_is_rejected() {
    let err = generate_err("struct Foo { @range(5, 1) n: u8 }");
    assert!(
        err.to_string()
            .contains("@range minimum 5 is greater than maximum 1")
    );
}
#[test]
fn lower_bool_rejects_numeric_logic_operand() {
    let ast = bytesmith_dsl_parse::parse_str("struct Foo { c = n == 1 && 2 }").unwrap();
    let err = crate::expr::lower(
        constraint_expr(&ast),
        crate::expr::ExprType::Bool,
        &numeric_done_fields(),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("is a number but a boolean is required")
    );
}
