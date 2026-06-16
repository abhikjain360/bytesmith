use super::*;

#[test]
fn golden_empty_struct() {
    assert_generated_eq(r#"struct Empty {}"#, "empty_struct");
}
#[test]
fn golden_nested_structs() {
    assert_generated_eq(
        r#"struct Inner { a: u8, b: u8 } struct Outer { prefix: u16, inner: Inner, suffix: u16 }"#,
        "nested_structs",
    );
}
#[test]
fn golden_concat() {
    assert_generated_eq(
        r#"struct WithConcat { flags: b<3>, fragment_offset: concat(b<5>, u8) }"#,
        "concat",
    );
}
#[test]
fn duplicate_struct_is_rejected() {
    let err = generate_err("struct Dup { a: u8 } struct Dup { b: u16 }");
    assert!(matches!(err, Error::DuplicateStruct { .. }));
}
#[test]
fn unknown_struct_reference_is_rejected() {
    let err = generate_err("struct Foo { inner: Bar }");
    assert!(matches!(err, Error::UnknownStruct { name } if name == "Bar"));
}
#[test]
fn dependency_cycle_is_rejected() {
    let err = generate_err("struct A { b: B } struct B { a: A }");
    assert!(matches!(err, Error::DependencyCycle { .. }));
}
#[test]
fn generation_is_deterministic() {
    let dsl = r#"
struct A { x: u8 }
struct B { x: u16 }
struct C { x: u32 }
struct D { a: A, b: B }
struct E { c: C, d: D }
struct F { e: E }
struct G { a: A, f: F }
struct H { g: G }
"#;
    let first = generate(dsl);
    for _ in 0..4 {
        assert_eq!(generate(dsl), first);
    }
}
