use super::*;

#[test]
fn golden_simple_primitive() {
    assert_generated_eq(r#"struct Simple { value: u32 }"#, "simple_primitive");
}
#[test]
fn golden_bitfields() {
    assert_generated_eq(
        r#"struct IpFlags { version: b<4>, ihl: b<4>, dscp: b<6>, ecn: b<2> }"#,
        "bitfields",
    );
}
#[test]
fn golden_cross_byte_bitfield() {
    assert_generated_eq(
        r#"struct Cross { a: b<5>, b: b<6>, c: b<5> }"#,
        "cross_byte_bitfield",
    );
}
#[test]
fn golden_endian_attributes() {
    assert_generated_eq(
        r#"@endian(little)
        struct LittlePacket {
            header: u32,
            @endian(big) mixed: u16,
            data: u8,
        }"#,
        "endian_attributes",
    );
}
#[test]
fn golden_signed_primitives() {
    assert_generated_eq(
        r#"@endian(little) struct SignedPrim { a: i8, b: i16, @endian(big) c: i32 }"#,
        "signed_primitives",
    );
}
#[test]
fn golden_lsb_bit_order() {
    assert_generated_eq(
        r#"@bit_order(lsb) struct LsbBits { low: b<3>, high: b<5> }"#,
        "lsb_bit_order",
    );
}
#[test]
fn endian_on_u8_is_rejected() {
    let err = generate_err("struct Foo { @endian(little) a: u8 }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to single-byte integers")
    );
}
#[test]
fn endian_on_bitfield_is_rejected() {
    let err = generate_err("struct Foo { @endian(little) a: b<4> }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to bitfields")
    );
}
#[test]
fn endian_on_struct_ref_is_rejected() {
    let err = generate_err("struct Inner { x: u8 } struct Foo { @endian(big) inner: Inner }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to struct ref")
    );
}
#[test]
fn invalid_endian_value_is_rejected() {
    let err = generate_err("struct Foo { @endian(middle) a: u16 }");
    assert!(err.to_string().contains("@endian argument must be"));
}
#[test]
fn endian_on_i8_is_rejected() {
    let err = generate_err("struct Foo { @endian(little) a: i8 }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to single-byte integers")
    );
}
#[test]
fn endian_on_single_byte_array_is_rejected() {
    let err = generate_err("struct Foo { @endian(little) xs: [u8; 2] }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to single-byte integers")
    );
}
#[test]
fn endian_on_bitfield_array_is_rejected() {
    let err = generate_err("struct Foo { @endian(little) xs: [b<4>; 2] }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to bitfields")
    );
}
#[test]
fn endian_on_struct_ref_array_is_rejected() {
    let err = generate_err("struct Inner { x: u8 } struct Foo { @endian(big) xs: [Inner; 2] }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to struct ref")
    );
}
#[test]
fn bit_order_on_primitive_is_rejected() {
    let err = generate_err("struct Foo { @bit_order(lsb) a: u16 }");
    assert!(
        err.to_string()
            .contains("@bit_order can only be applied to bitfields")
    );
}
#[test]
fn bit_order_on_struct_ref_is_rejected() {
    let err = generate_err("struct Inner { x: u8 } struct Foo { @bit_order(lsb) inner: Inner }");
    assert!(
        err.to_string()
            .contains("@bit_order can only be applied to bitfields")
    );
}
#[test]
fn invalid_bit_order_value_is_rejected() {
    let err = generate_err("struct Foo { @bit_order(big) a: b<4> }");
    assert!(err.to_string().contains("@bit_order argument must be"));
}
#[test]
fn endian_wrong_arg_count_is_rejected() {
    let err = generate_err("struct Foo { @endian(big, little) a: u16 }");
    assert!(err.to_string().contains("requires exactly 1 argument"));
}
