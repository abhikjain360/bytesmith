fn generate_code(dsl: &str) -> String {
    let ast = binparse_dsl_parse::parse_str(dsl).expect("failed to parse DSL");
    binparse_codegen::CodeGen::generate(&ast).expect("failed to generate code")
}

#[test]
fn test_generated_code_is_valid_rust() {
    let dsl = r#"
struct WithFixedHook {
    prefix: u8,
    @hook(double_it, u32)
    value: u16,
    suffix: u8,
}

struct WithVlaHook {
    len: u8,
    @hook(parse_cstring, String)
    name: [u8],
}
"#;

    let code = generate_code(dsl);
    let parsed: syn::File = syn::parse_str(&code).expect("generated code is not valid Rust");

    assert!(
        parsed.items.len() >= 2,
        "expected at least 2 items (structs + impls)"
    );
}

#[test]
fn test_fixed_hook_generates_correct_pattern() {
    let dsl = r#"
struct Test {
    @hook(my_hook, MyType)
    field: u32,
}
"#;

    let code = generate_code(dsl);

    assert!(code.contains("pub fn field(&self) -> MyType"), "should have getter returning hook's return type");
    assert!(code.contains("my_hook("), "should call the hook function");
}

#[test]
fn test_vla_hook_generates_correct_pattern() {
    let dsl = r#"
struct Test {
    prefix: u8,
    @hook(my_vla_hook, MyResult)
    data: [u8],
}
"#;

    let code = generate_code(dsl);

    assert!(code.contains("fn data_raw(&self) -> (MyResult, usize)"), "should have raw helper");
    assert!(code.contains("pub fn data(&self) -> MyResult"), "should have public getter");
    assert!(code.contains("self.data_raw().0"), "getter should return first element of tuple");
    assert!(code.contains("self.data_raw().1"), "offset should use second element of tuple");
    assert!(code.contains("my_vla_hook(&self.data["), "should call hook with slice");
}

#[test]
fn test_vla_hook_at_start() {
    let dsl = r#"
struct Test {
    @hook(first_hook, String)
    first: [u8],
}
"#;

    let code = generate_code(dsl);

    assert!(code.contains("first_hook(&self.data[0..])"), "should start at offset 0 when first field");
}

#[test]
fn test_fixed_hook_preserves_length() {
    let dsl = r#"
struct Test {
    @hook(transform, Result)
    a: u32,
    b: u16,
}
"#;

    let code = generate_code(dsl);

    assert!(code.contains("byte: 4usize"), "a should still be 4 bytes");
    assert!(code.contains("byte: 6usize"), "b should start at offset 4 + end at 6");
}

#[test]
fn test_vla_hook_dynamic_length() {
    let dsl = r#"
struct Test {
    prefix: u8,
    @hook(my_hook, String)
    data: [u8],
}
"#;

    let code = generate_code(dsl);

    assert!(code.contains("self.data_raw().1"), "should use hook's returned length");
    assert!(code.contains("byte: self.data_raw().1"), "offset calculation should include dynamic length");
}
