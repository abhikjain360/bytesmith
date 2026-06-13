const BASELINE_DSL: &str = r#"
struct Inner {
    a: u8,
    b: u16,
}

@endian(little)
struct Baseline {
    n: u8,
    word: u16,
    @endian(big) be: u32,
    flag_a: b<3>,
    flag_b: b<5>,
    fixed: [u8; 3],
    inner: Inner,
    dyns: [u16; n],
    pair: concat(u8, u16),
    payload: union(n) {
        1 => One { x: u8 },
        _ => Unknown { },
    },
}

struct Hooked {
    prefix: u8,
    @hook(double_it, u32)
    value: u16,
    @hook(parse_cstring, String)
    name: [u8],
}

struct StructArray {
    count: u8,
    items: [Inner; count],
}

struct SizeExpr {
    n: u64,
    xs: [u8; n * 2],
}

@endian(little)
struct Mixed {
    a: i8,
    b: i16,
    @endian(big) c: i64,
    version: b<4>,
    ihl: b<4>,
    @bit_order(lsb) low: b<3>,
    @bit_order(lsb) high: b<5>,
    vals: [i16; ihl],
}
"#;

fn main() {
    let ast = binparse_dsl_parse::parse_str(BASELINE_DSL).expect("failed to parse baseline DSL");
    let code = binparse_codegen::CodeGen::generate(&ast).expect("failed to generate baseline code");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    std::fs::write(std::path::Path::new(&out_dir).join("generated.rs"), code)
        .expect("failed to write generated code");
}
