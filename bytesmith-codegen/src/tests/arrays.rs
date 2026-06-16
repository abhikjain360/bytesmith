use super::*;

#[test]
fn golden_fixed_array() {
    assert_generated_eq(
        r#"struct WithArray { count: u8, data: [u32; 4] }"#,
        "fixed_array",
    );
}
#[test]
fn golden_expression_sized_array() {
    assert_generated_eq(
        r#"struct ExprArray { n: u8, items: [u16; n * 2] }"#,
        "expression_sized_array",
    );
}
#[test]
fn golden_greedy_rest_array() {
    assert_generated_eq(
        r#"struct Rest { n: u8, @greedy(unsafe_eof) tail: [u8] }"#,
        "greedy_rest_array",
    );
}
#[test]
fn golden_until_array() {
    assert_generated_eq(
        r#"struct CStr { @until(x00) name: [u8], after: u8 }"#,
        "until_array",
    );
}
#[test]
fn golden_struct_ref_array() {
    assert_generated_eq(
        r#"struct Inner { a: u8 } struct StructArray { count: u8, items: [Inner; count] }"#,
        "struct_ref_array",
    );
}
#[test]
fn golden_bitfield_array() {
    assert_generated_eq(
        r#"struct BitArray { nibbles: [b<4>; 4] }"#,
        "bitfield_array",
    );
}
#[test]
fn array_size_unknown_field_is_rejected() {
    let err = generate_err("struct Foo { items: [u16; nope] }");
    assert!(
        err.to_string().contains(
            "expression 'nope' references field 'nope' which is unknown or not yet parsed"
        )
    );
}
#[test]
fn array_size_non_numeric_field_is_rejected() {
    let err =
        generate_err("struct Inner { x: u8 } struct Foo { inner: Inner, items: [u8; inner] }");
    assert!(
        err.to_string()
            .contains("expression 'inner' references field 'inner' which is not a numeric field")
    );
}
#[test]
fn array_size_forward_reference_is_rejected() {
    let err = generate_err("struct Foo { data: [u8; later], later: u8 }");
    assert!(
        err.to_string()
            .contains("references field 'later' which is unknown or not yet parsed")
    );
}
#[test]
fn array_size_bool_expr_is_rejected() {
    let err = generate_err("struct Foo { n: u8, data: [u8; n == 1] }");
    assert!(
        err.to_string()
            .contains("is a boolean but a number is required")
    );
}
#[test]
fn array_size_string_is_rejected() {
    let err = generate_err(r#"struct Foo { data: [u8; "two"] }"#);
    assert!(
        err.to_string()
            .contains("is a string but a number is required")
    );
}
#[test]
fn array_size_nested_path_is_rejected() {
    let err = generate_err("struct Foo { a: u8, data: [u8; a.len] }");
    assert!(err.to_string().contains("nested path 'a.len'"));
}
#[test]
fn array_size_const_overflow_is_rejected() {
    let err = generate_err("struct Foo { data: [u8; 4294967296 * 4294967296] }");
    assert!(err.to_string().contains("overflows"));
}
#[test]
fn array_size_division_by_zero_is_rejected() {
    let err = generate_err("struct Foo { data: [u8; 8 / 0] }");
    assert!(err.to_string().contains("divides by zero"));
}
#[test]
fn array_size_dynamic_divisor_is_rejected() {
    let err = generate_err("struct Foo { n: u8, data: [u8; 8 / n] }");
    assert!(err.to_string().contains("divides by a non-constant value"));
}
#[test]
fn same_array_field_name_in_two_structs_does_not_collide() {
    let code =
        generate("struct First { n: u8, xs: [u8; n] } struct Second { n: u8, xs: [u16; n] }");
    assert!(code.contains("First_xs_Iterator"));
    assert!(code.contains("Second_xs_Iterator"));
}
#[test]
fn array_size_const_expr_is_folded() {
    let code = generate("struct Foo { data: [u8; 2 * 3 + 1] }");
    assert!(code.contains("count: 7usize"));
}
#[test]
fn array_size_subtraction_saturates() {
    let code = generate("struct Foo { n: u8, data: [u8; n - 1] }");
    assert!(code.contains("(self.n() as usize).saturating_sub(1usize)"));
}
#[test]
fn unsized_array_without_strategy_is_rejected() {
    let err = generate_err("struct Foo { data: [u8] }");
    assert!(
        err.to_string()
            .contains("array without size requires @until, @greedy, or @hook")
    );
}
#[test]
fn until_on_sized_array_is_rejected() {
    let err = generate_err("struct Foo { @until(x00) data: [u8; 4] }");
    assert!(
        err.to_string()
            .contains("@until requires an array without an explicit size")
    );
}
#[test]
fn greedy_on_sized_array_is_rejected() {
    let err = generate_err("struct Foo { @greedy(unsafe_eof) data: [u8; 4] }");
    assert!(
        err.to_string()
            .contains("@greedy requires an array without an explicit size")
    );
}
#[test]
fn until_on_non_array_is_rejected() {
    let err = generate_err("struct Foo { @until(x00) data: u8 }");
    assert!(
        err.to_string()
            .contains("@until can only be applied to array fields")
    );
}
#[test]
fn max_iter_on_non_array_is_rejected() {
    let err = generate_err("struct Foo { @max_iter(4) data: u8 }");
    assert!(
        err.to_string()
            .contains("@max_iter can only be applied to array fields")
    );
}
#[test]
fn until_with_greedy_is_rejected() {
    let err = generate_err("struct Foo { @until(x00) @greedy(unsafe_eof) data: [u8] }");
    assert!(
        err.to_string()
            .contains("@until and @greedy cannot be combined")
    );
}
#[test]
fn greedy_with_hook_is_rejected() {
    let err = generate_err("struct Foo { @hook(f, u8) @greedy(unsafe_eof) data: [u8] }");
    assert!(
        err.to_string()
            .contains("@greedy cannot be combined with @hook")
    );
}
#[test]
fn hook_on_non_u8_vla_is_rejected() {
    let err = generate_err("struct Foo { @hook(f, u8) data: [u16] }");
    assert!(err.to_string().contains("@hook on VLA requires [u8] type"));
}
#[test]
fn invalid_greedy_value_is_rejected() {
    let err = generate_err("struct Foo { @greedy(eof) data: [u8] }");
    assert!(
        err.to_string()
            .contains("@greedy argument must be 'unsafe_eof', got 'eof'")
    );
}
#[test]
fn until_sentinel_too_wide_is_rejected() {
    let err = generate_err("struct Foo { @until(x0100) data: [u8] }");
    assert!(
        err.to_string()
            .contains("@until sentinel must be an integer literal fitting in one byte")
    );
}
#[test]
fn greedy_dynamic_elem_without_max_iter_is_rejected() {
    let err = generate_err(
        "struct Opt { kind: u8, if (kind > 0) { body: u8 } } struct Foo { @greedy(unsafe_eof) opts: [Opt] }",
    );
    assert!(
        err.to_string()
            .contains("@greedy with dynamic-length elements requires @max_iter")
    );
}
#[test]
fn greedy_zero_sized_elem_is_rejected() {
    let err = generate_err("struct Empty { } struct Foo { @greedy(unsafe_eof) xs: [Empty] }");
    assert!(
        err.to_string()
            .contains("@greedy element type has zero length")
    );
}
