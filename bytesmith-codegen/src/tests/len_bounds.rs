use super::*;

#[test]
fn golden_len_bounded_struct_ref() {
    assert_generated_eq(
        r#"struct Inner { a: u8, b: u16 }
         struct Tlv { tag: u8, len: u8, @len(len) value: Inner, after: u8 }"#,
        "len_bounded_struct_ref",
    );
}
#[test]
fn len_on_primitive_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @len(n) a: u8 }");
    assert!(
        err.to_string()
            .contains("@len can only be applied to struct ref, union, or unsized array fields")
    );
}
#[test]
fn len_on_bitfield_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @len(n) a: b<4> }");
    assert!(
        err.to_string()
            .contains("@len can only be applied to struct ref, union, or unsized array fields")
    );
}
#[test]
fn len_on_constant_field_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @len(n) magic = xff }");
    assert!(
        err.to_string()
            .contains("@len can only be applied to struct ref, union, or unsized array fields")
    );
}
#[test]
fn len_on_counted_array_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @len(n) a: [u8; n] }");
    assert!(
        err.to_string()
            .contains("@len cannot be applied to a counted or expression-sized array")
    );
}
#[test]
fn len_on_bitfield_array_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @len(n) @greedy(unsafe_eof) a: [b<4>] }");
    assert!(
        err.to_string()
            .contains("@len cannot be applied to a bitfield-element array")
    );
}
#[test]
fn len_on_greedy_array_is_accepted() {
    let code = generate("struct Foo { n: u8, @len(n) @greedy(unsafe_eof) a: [u8] }");
    assert!(code.contains("fn a_rest"));
}
#[test]
fn len_on_until_array_is_accepted() {
    let code = generate("struct Foo { n: u8, @len(n) @until(x00) a: [u8] }");
    assert!(code.contains("fn a_rest"));
}
#[test]
fn len_on_union_is_accepted() {
    let code = generate(
        "struct Inner { a: u8 }
         struct Foo { t: u8, n: u8, @len(n) v: union(t) { 1 => A { i: Inner }, _ => B { } } }",
    );
    assert!(code.contains("fn v_rest"));
}
#[test]
fn len_wrong_arg_count_is_rejected() {
    let err = generate_err(
        "struct Inner { a: u8 }
         struct Foo { n: u8, @len(n, 2) value: Inner }",
    );
    assert!(
        err.to_string()
            .contains("@len requires exactly 1 argument(s), got 2")
    );
}
#[test]
fn len_bound_smaller_than_fixed_inner_is_rejected() {
    let err = generate_err(
        "struct Inner { a: u32 }
         struct Foo { @len(2) value: Inner }",
    );
    assert!(
        err.to_string()
            .contains("@len(2) is smaller than the referenced struct's fixed length of 4 bytes")
    );
}
#[test]
fn len_unknown_field_is_rejected() {
    let err = generate_err(
        "struct Inner { a: u8 }
         struct Foo { @len(nope) value: Inner }",
    );
    assert!(err.to_string().contains("references field 'nope'"));
}
#[test]
fn len_bound_equal_to_fixed_inner_is_accepted() {
    let code = generate(
        "struct Inner { a: u32 }
         struct Foo { @len(4) value: Inner }",
    );
    assert!(code.contains("fn value_rest"));
}
#[test]
fn struct_level_len_fill_to_bound_array_is_accepted() {
    let code = generate("@len(total_len) struct Foo { total_len: u16, payload: [u8] }");
    assert!(code.contains("fn struct_len"));
}
#[test]
fn bare_sizeless_array_without_struct_len_is_rejected() {
    let err = generate_err("struct Foo { total_len: u16, payload: [u8] }");
    assert!(
        err.to_string()
            .contains("array without size requires @until, @greedy, or @hook")
    );
}
#[test]
fn fill_to_bound_array_not_last_is_rejected() {
    let err =
        generate_err("@len(total_len) struct Foo { total_len: u16, payload: [u8], tail: u8 }");
    assert!(
        err.to_string()
            .contains("fill-to-bound array field 'payload' must be the last field in the struct")
    );
}
#[test]
fn struct_level_len_on_conditional_field_is_rejected() {
    let err = generate_err("@len(n) struct Foo { f: u8, if (f > 0) { n: u8 } payload: [u8] }");
    assert!(err.to_string().contains("references conditional field 'n'"));
}
#[test]
fn struct_level_len_wrong_arg_count_is_rejected() {
    let err = generate_err("@len(total_len, 2) struct Foo { total_len: u16, payload: [u8] }");
    assert!(
        err.to_string()
            .contains("@len requires exactly 1 argument(s), got 2")
    );
}
#[test]
fn golden_struct_level_len() {
    assert_generated_eq(
        r#"@len(len) struct Bounded { len: u8, value: u16 }"#,
        "struct_level_len",
    );
}
#[test]
fn golden_struct_level_len_fill_to_bound() {
    assert_generated_eq(
        r#"@len(total_len) struct Filled { total_len: u16, payload: [u8] }"#,
        "struct_level_len_fill_to_bound",
    );
}
#[test]
fn golden_len_bounded_union() {
    assert_generated_eq(
        r#"
struct Inner { a: u8, b: u16 }
struct Tlv { tag: u8, len: u8, @len(len) value: union(tag) { 1 => Addr { inner: Inner }, _ => Raw { @greedy(unsafe_eof) bytes: [u8] } }, after: u8 }
"#,
        "len_bounded_union",
    );
}
#[test]
fn golden_len_bounded_greedy_array() {
    assert_generated_eq(
        r#"
struct Frame { tag: u8, len: u8, @len(len) @greedy(unsafe_eof) body: [u8], after: u8 }
"#,
        "len_bounded_greedy_array",
    );
}
