use super::*;

#[test]
fn golden_conditional_fields() {
    assert_generated_eq(
        r#"
        struct Cond {
            n: u8,
            if (n == 1) {
                x: u16,
            } else {
                y: u8,
            }
            tail: u8,
        }
        "#,
        "conditional_fields",
    );
}
#[test]
fn conditional_field_reference_is_rejected() {
    let err = generate_err("struct Foo { n: u8, if (n == 1) { m: u8 } data: [u8; m] }");
    assert!(
        err.to_string()
            .contains("expression 'm' references conditional field 'm' which may be absent")
    );
}
#[test]
fn conditional_intra_branch_reference_is_rejected() {
    let err = generate_err("struct Foo { n: u8, if (n == 1) { m: u8, data: [u8; m] } }");
    assert!(
        err.to_string()
            .contains("expression 'm' references conditional field 'm' which may be absent")
    );
}
#[test]
fn conditional_numeric_condition_is_rejected() {
    let err = generate_err("struct Foo { n: u8, if (n) { m: u8 } }");
    assert!(
        err.to_string()
            .contains("expression 'n' is a number but a boolean is required")
    );
}
#[test]
fn conditional_forward_reference_is_rejected() {
    let err = generate_err("struct Foo { if (later == 1) { m: u8 } later: u8 }");
    assert!(err.to_string().contains(
        "expression '(later == 1)' references field 'later' which is unknown or not yet parsed"
    ));
}
#[test]
fn golden_padding_and_alignment() {
    assert_generated_eq(
        r#"struct Padded { a: u8, @pad(2) b: u8, @pad_to(4) c: u16, @align(2) d: u16 }"#,
        "padding_and_alignment",
    );
}
#[test]
fn golden_skip_fields() {
    assert_generated_eq(
        r#"struct Skipped { @skip reserved: b<3>, flags: b<5>, pair: concat(b<4>, @skip b<4>) }"#,
        "skip_fields",
    );
}
#[test]
fn align_on_misaligned_fixed_offset_is_rejected() {
    let err = generate_err("struct Foo { a: u8, @align(2) b: u8 }");
    assert!(
        err.to_string()
            .contains("@align(2) field starts at misaligned offset")
    );
}
#[test]
fn align_on_unaligned_bit_offset_is_rejected() {
    let err = generate_err("struct Foo { a: b<3>, @align(1) b: u8 }");
    assert!(
        err.to_string()
            .contains("@align(1) field starts at misaligned offset")
    );
}
#[test]
fn pad_with_pad_to_is_rejected() {
    let err = generate_err("struct Foo { @pad(1) @pad_to(4) a: u8 }");
    assert!(
        err.to_string()
            .contains("@pad and @pad_to cannot be combined")
    );
}
#[test]
fn zero_padding_arg_is_rejected() {
    let err = generate_err("struct Foo { @align(0) a: u8 }");
    assert!(
        err.to_string()
            .contains("@align argument must be a positive integer literal")
    );
}
#[test]
fn non_literal_padding_arg_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @pad(n) a: u8 }");
    assert!(
        err.to_string()
            .contains("@pad argument must be a positive integer literal")
    );
}
#[test]
fn skip_with_args_is_rejected() {
    let err = generate_err("struct Foo { @skip(1) a: u8 }");
    assert!(
        err.to_string()
            .contains("@skip requires exactly 0 argument(s), got 1")
    );
}
#[test]
fn pad_wrong_arg_count_is_rejected() {
    let err = generate_err("struct Foo { @pad_to(1, 2) a: u8 }");
    assert!(
        err.to_string()
            .contains("@pad_to requires exactly 1 argument(s), got 2")
    );
}
