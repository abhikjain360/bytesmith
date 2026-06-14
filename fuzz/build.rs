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

struct Validated {
    magic = x89504e47,
    @check(version == 4) version: b<4>,
    ihl: b<4>,
    @range(20, 60) total_len: u16,
    reserved = b00,
    @check(flags <= 3) flags: b<6>,
}

struct Conditional {
    version: b<4>,
    ihl: b<4>,
    if (ihl > 5) {
        options: [u8; (ihl - 5) * 4],
    } else {
        big: u16,
    }
    tail: u8,
}

struct Rest {
    n: u8,
    @greedy(unsafe_eof) words: [u16],
}

struct CStr {
    @until(x00) name: [u8],
    after: u8,
}

struct Capped {
    count: u8,
    @max_iter(4) vals: [u8; count],
}

struct Opt {
    kind: u8,
    if (kind > 0) {
        body: u8,
    }
}

struct Opts {
    @greedy(unsafe_eof) @max_iter(8) opts: [Opt],
}

struct Padded {
    @skip reserved: b<3>,
    flags: b<5>,
    n: u8,
    @pad(1) data: [u8; n],
    @pad_to(4) @align(2) tail: u16,
}

error {
    UNKNOWN_KIND { kind: u8 },
}

struct Dispatch {
    kind: u8,
    body: union(kind) {
        1 => Msg { msg_len: u8, data: [u8; msg_len] },
        2 => Checked { version = 4 },
        _ => @error(UNKNOWN_KIND { kind: kind }),
    },
}

struct ConcatUnion {
    a: u8,
    b: u8,
    pair: concat(
        u8,
        union(a) { 1 => Word { w: u16 }, _ => Empty { } },
        union(b) { 2 => Bytes { count: u8, data: [u8; count] }, _ => Skip { } }
    ),
    tail: u8,
}

struct Bounded {
    tag: u8,
    length: u8,
    @len(length) value: Inner,
    after: u8,
}

struct BoundedUnion {
    tag: u8,
    length: u8,
    @len(length) value: union(tag) {
        1 => Pair { inner: Inner },
        2 => Blob { @greedy(unsafe_eof) bytes: [u8] },
        _ => Unknown { },
    },
    after: u8,
}

struct Varint {
    tag: u8,
    @hook(read_leb128, u64) value: [u8],
    after: u8,
}

struct LenVarint {
    n: u8,
    @len(n) @hook(read_leb128, u64) value: [u8],
    after: u8,
}

struct DnsMsg {
    id: u16,
    @hook(parse_dns_name, String) qname: [u8],
    qtype: u16,
    @hook(parse_dns_name, String) aname: [u8],
    atype: u16,
}

@len(total_len)
struct StructBounded {
    total_len: u16,
    payload: [u8],
}

@len(n)
struct StructBoundedInner {
    n: u8,
    @greedy(unsafe_eof) body: [u8],
}

struct StructBoundedNested {
    inner: StructBoundedInner,
    after: u8,
}
"#;

const WRITER_BASELINE_DSL: &str = r#"
@endian(little)
struct WPrim {
    a: u8,
    word: u16,
    @endian(big) be: u32,
    sword: i16,
}

@endian(big)
struct WBits {
    flag_a: b<4>,
    flag_b: b<4>,
    ttl: u8,
    total: u16,
}

struct WFixedArr {
    tag: u8,
    bytes: [u8; 4],
    tail: u16,
}

@endian(big)
struct WInner {
    a: u8,
    b: u16,
}

@endian(big)
struct WNested {
    tag: u8,
    inner: WInner,
    trailer: u8,
}

struct WLenTail {
    kind: u8,
    len: u8,
    payload: [u8; len],
}

struct WVarint {
    tag: u8,
    @hook(read_leb128, u64) @write_hook(binparse.hooks.write_leb128_unsigned, binparse.hooks.leb128_unsigned_len) len: [u8],
    body: [u8; len],
}

@endian(big)
struct WUnion {
    kind: u8,
    body: union(kind) {
        1 => WConnect { keep_alive: u16 },
        2 => WConnack { ack: u8, code: u8 },
        _ => Unknown { },
    },
}

struct WEthernet {
    dst: [u8; 6],
    src: [u8; 6],
    @discriminator ethertype: u16,
    @greedy(unsafe_eof) @payload payload: [u8],
}

struct WVlan {
    dst: [u8; 6],
    src: [u8; 6],
    tpid = x8100,
    pcp: b<3>,
    dei: b<1>,
    vid_hi: b<4>,
    vid_lo: u8,
    ethertype: u16,
    @greedy(unsafe_eof) payload: [u8],
}
"#;

fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");

    let ast = binparse_dsl_parse::parse_str(BASELINE_DSL).expect("failed to parse baseline DSL");
    let code = binparse_codegen::CodeGen::generate(&ast).expect("failed to generate baseline code");
    std::fs::write(std::path::Path::new(&out_dir).join("generated.rs"), code)
        .expect("failed to write generated code");

    let writer_ast = binparse_dsl_parse::parse_str(WRITER_BASELINE_DSL)
        .expect("failed to parse writer baseline DSL");
    let writer_code = binparse_codegen::CodeGen::generate_writers(&writer_ast)
        .expect("failed to generate writer baseline code");
    std::fs::write(
        std::path::Path::new(&out_dir).join("generated_writers.rs"),
        writer_code,
    )
    .expect("failed to write generated writer code");
}
