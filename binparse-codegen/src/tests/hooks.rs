use super::*;

#[test]
fn golden_fixed_hook() {
    assert_generated_eq(
        r#"struct WithFixedHook {
            prefix: u8,
            @hook(double_it, u32)
            value: u16,
            suffix: u8,
        }"#,
        "fixed_hook",
    );
}
#[test]
fn golden_len_bounded_hook() {
    assert_generated_eq(
        r#"struct WithLenHook {
            len: u8,
            @len(len) @hook(read_leb128, u64) value: [u8],
            after: u8,
        }"#,
        "len_bounded_hook",
    );
}
#[test]
fn golden_conditional_hook() {
    assert_generated_eq(
        r#"struct WithCondHook {
            kind: u8,
            if (kind == 1) {
                @hook(double_it, u32) value: u16,
            }
            tail: u8,
        }"#,
        "conditional_hook",
    );
}
#[test]
fn golden_vla_hook() {
    assert_generated_eq(
        r#"struct WithVlaHook {
            len: u8,
            @hook(parse_cstring, String)
            name: [u8],
        }"#,
        "vla_hook",
    );
}
