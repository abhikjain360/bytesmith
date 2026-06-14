use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend,
};

use binparse_dsl as ast;

const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::TYPE,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::DECORATOR,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::NUMBER,
    SemanticTokenType::STRING,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::COMMENT,
    SemanticTokenType::ENUM_MEMBER,
];

const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[SemanticTokenModifier::DECLARATION];

const TYPE: u32 = 0;
const PROPERTY: u32 = 1;
const VARIABLE: u32 = 2;
const DECORATOR: u32 = 3;
const FUNCTION: u32 = 4;
const NUMBER: u32 = 5;
const STRING: u32 = 6;
const OPERATOR: u32 = 7;
const KEYWORD: u32 = 8;
const COMMENT: u32 = 9;
const ENUM_MEMBER: u32 = 10;

const DECL: u32 = 1;

const KEYWORDS: [&str; 7] = ["struct", "union", "concat", "if", "else", "error", "match"];

pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

/// Build the full semantic-token set for a `.bp` source string. The structural
/// tokens come from the spanned AST; keywords and comments (which never reach
/// the AST) come from a linear scan. If the source fails to parse, the AST
/// tokens are skipped but the lexical scan still runs.
pub fn compute(text: &str) -> Vec<SemanticToken> {
    let mut raws = Vec::new();
    if let Ok(defs) = binparse_dsl_parse::parse_str_located(text) {
        for def in &defs {
            visit_definition(&mut raws, def);
        }
    }
    scan_lexical(text, &mut raws);
    encode(text, raws)
}

#[derive(Clone, Copy)]
struct Raw {
    start: u32,
    end: u32,
    ty: u32,
    mods: u32,
}

fn push(raws: &mut Vec<Raw>, span: ast::Span, ty: u32, mods: u32) {
    if span.end > span.start {
        raws.push(Raw {
            start: span.start,
            end: span.end,
            ty,
            mods,
        });
    }
}

fn visit_definition(raws: &mut Vec<Raw>, def: &ast::Definition) {
    match def {
        ast::Definition::Struct(s) => visit_struct(raws, s),
        ast::Definition::Error(variants) => {
            for v in variants {
                push(raws, v.name.span, ENUM_MEMBER, DECL);
                for (name, _) in &v.fields {
                    push(raws, name.span, PROPERTY, DECL);
                }
            }
        }
    }
}

fn visit_struct(raws: &mut Vec<Raw>, s: &ast::Struct) {
    for a in &s.attributes {
        visit_attribute(raws, a);
    }
    push(raws, s.name.span, TYPE, DECL);
    for item in &s.items {
        visit_struct_item(raws, item);
    }
}

fn visit_attribute(raws: &mut Vec<Raw>, a: &ast::Attribute) {
    push(raws, a.name.span, DECORATOR, 0);
    for e in &a.args {
        visit_expr(raws, e);
    }
}

fn visit_struct_item(raws: &mut Vec<Raw>, item: &ast::StructItem) {
    match item {
        ast::StructItem::Field(f) => visit_field(raws, f),
        ast::StructItem::Conditional(c) => visit_conditional(raws, c),
    }
}

fn visit_field(raws: &mut Vec<Raw>, f: &ast::Field) {
    for a in &f.attributes {
        visit_attribute(raws, a);
    }
    push(raws, f.name.span, PROPERTY, DECL);
    match &f.value {
        ast::FieldValue::Type(t) => visit_type(raws, t),
        ast::FieldValue::Constraint(e) => visit_expr(raws, e),
    }
}

fn visit_conditional(raws: &mut Vec<Raw>, c: &ast::Conditional) {
    visit_expr(raws, &c.condition);
    for item in &c.then_branch {
        visit_struct_item(raws, item);
    }
    if let Some(else_branch) = &c.else_branch {
        for item in else_branch {
            visit_struct_item(raws, item);
        }
    }
}

fn visit_type(raws: &mut Vec<Raw>, t: &ast::Type) {
    match &t.kind {
        ast::TypeKind::BitField(_) | ast::TypeKind::Primitive(_) => push(raws, t.span, TYPE, 0),
        ast::TypeKind::Array(arr) => {
            visit_array_elem(raws, &arr.elem_ty);
            if let Some(size) = &arr.size {
                visit_expr(raws, size);
            }
        }
        ast::TypeKind::StructRef(name) => push(raws, name.span, TYPE, 0),
        ast::TypeKind::Concat(items) => {
            for ci in items {
                for a in &ci.attributes {
                    visit_attribute(raws, a);
                }
                visit_type(raws, &ci.ty);
            }
        }
        ast::TypeKind::Union(u) => {
            for arg in &u.args {
                push(raws, arg.span, VARIABLE, 0);
            }
            for v in &u.variants {
                visit_union_variant(raws, v);
            }
        }
    }
}

fn visit_array_elem(raws: &mut Vec<Raw>, e: &ast::ArrayElemType) {
    match &e.kind {
        ast::ArrayElemTypeKind::BitField(_) | ast::ArrayElemTypeKind::Primitive(_) => {
            push(raws, e.span, TYPE, 0)
        }
        ast::ArrayElemTypeKind::StructRef(name) => push(raws, name.span, TYPE, 0),
    }
}

fn visit_union_variant(raws: &mut Vec<Raw>, v: &ast::UnionVariant) {
    for m in &v.matchers {
        visit_union_matcher(raws, m);
    }
    match &v.body {
        ast::UnionBody::NamedInline(nis) => {
            for a in &nis.attributes {
                visit_attribute(raws, a);
            }
            push(raws, nis.name.span, TYPE, DECL);
            for item in &nis.items {
                visit_struct_item(raws, item);
            }
        }
        ast::UnionBody::Error(name, fields) => {
            push(raws, name.span, ENUM_MEMBER, 0);
            for (fname, e) in fields {
                push(raws, fname.span, PROPERTY, 0);
                visit_expr(raws, e);
            }
        }
    }
}

fn visit_union_matcher(raws: &mut Vec<Raw>, m: &ast::UnionMatcher) {
    match &m.kind {
        ast::UnionMatcherKind::Literal(lit) => visit_literal(raws, lit, m.span),
        ast::UnionMatcherKind::Wildcard => {}
        ast::UnionMatcherKind::Tuple(ms) => {
            for sub in ms {
                visit_union_matcher(raws, sub);
            }
        }
    }
}

fn visit_expr(raws: &mut Vec<Raw>, e: &ast::Expr) {
    match &e.kind {
        ast::ExprKind::Literal(lit) => visit_literal(raws, lit, e.span),
        ast::ExprKind::Path(segs) => {
            for seg in segs {
                push(raws, seg.span, VARIABLE, 0);
            }
        }
        ast::ExprKind::Binary(b) => {
            visit_expr(raws, &b.lhs);
            push(raws, b.op_span, OPERATOR, 0);
            visit_expr(raws, &b.rhs);
        }
        ast::ExprKind::Call(name, args) => {
            push(raws, name.span, FUNCTION, 0);
            for a in args {
                visit_expr(raws, a);
            }
        }
        ast::ExprKind::Tuple(es) => {
            for e in es {
                visit_expr(raws, e);
            }
        }
        ast::ExprKind::RawType(_) => push(raws, e.span, TYPE, 0),
    }
}

fn visit_literal(raws: &mut Vec<Raw>, lit: &ast::Literal, enclosing: ast::Span) {
    match lit {
        ast::Literal::Int(il) => push(raws, il.span, NUMBER, 0),
        ast::Literal::String(_) => push(raws, enclosing, STRING, 0),
    }
}

/// Linear scan for the lexical tokens the parser discards: keywords and
/// comments. String regions are consumed (so a keyword inside a string is not
/// matched) but not emitted here — the AST owns string literals.
fn scan_lexical(text: &str, raws: &mut Vec<Raw>) {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        let b = bytes[i];
        if b == b'/' && i + 1 < n && bytes[i + 1] == b'/' {
            let start = i;
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            raws.push(Raw {
                start: start as u32,
                end: i as u32,
                ty: COMMENT,
                mods: 0,
            });
        } else if b == b'/' && i + 1 < n && bytes[i + 1] == b'*' {
            let start = i;
            i += 2;
            while i + 1 < n && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            let end = if i + 1 < n { i + 2 } else { n };
            raws.push(Raw {
                start: start as u32,
                end: end as u32,
                ty: COMMENT,
                mods: 0,
            });
            i = end;
        } else if b == b'"' {
            i += 1;
            while i < n && bytes[i] != b'"' {
                i += 1;
            }
            if i < n {
                i += 1;
            }
        } else if is_ident_start(b) {
            let start = i;
            i += 1;
            while i < n && is_ident_continue(bytes[i]) {
                i += 1;
            }
            if KEYWORDS.contains(&&text[start..i]) {
                raws.push(Raw {
                    start: start as u32,
                    end: i as u32,
                    ty: KEYWORD,
                    mods: 0,
                });
            }
        } else {
            i += 1;
        }
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Convert collected byte-range tokens into the LSP relative-delta encoding.
/// Multi-line tokens (block comments, multi-line strings) are split per line,
/// everything is sorted, overlaps are dropped, and offsets become UTF-16 columns.
fn encode(text: &str, raws: Vec<Raw>) -> Vec<SemanticToken> {
    let mut expanded = Vec::new();
    for r in raws {
        split_lines(text, r, &mut expanded);
    }
    expanded.sort_by_key(|r| (r.start, r.end));

    let index = LineIndex::new(text);
    let mut out = Vec::new();
    let mut prev_line = 0;
    let mut prev_col = 0;
    let mut last_end = 0;
    for r in expanded {
        if r.start < last_end {
            continue;
        }
        let length = utf16_len(&text[r.start as usize..r.end as usize]);
        if length == 0 {
            continue;
        }
        let (line, line_start) = index.line_at(r.start);
        let col = utf16_len(&text[line_start as usize..r.start as usize]);
        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 { col - prev_col } else { col };
        out.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: r.ty,
            token_modifiers_bitset: r.mods,
        });
        prev_line = line;
        prev_col = col;
        last_end = r.end;
    }
    out
}

fn split_lines(text: &str, r: Raw, out: &mut Vec<Raw>) {
    let bytes = text.as_bytes();
    let mut s = r.start;
    while s < r.end {
        let mut e = s;
        while e < r.end && bytes[e as usize] != b'\n' {
            e += 1;
        }
        if e > s {
            out.push(Raw {
                start: s,
                end: e,
                ty: r.ty,
                mods: r.mods,
            });
        }
        s = e + 1;
    }
}

fn utf16_len(s: &str) -> u32 {
    s.chars().map(|c| c.len_utf16() as u32).sum()
}

struct LineIndex {
    starts: Vec<u32>,
}

impl LineIndex {
    fn new(text: &str) -> LineIndex {
        let mut starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push((i + 1) as u32);
            }
        }
        LineIndex { starts }
    }

    fn line_at(&self, offset: u32) -> (u32, u32) {
        match self.starts.binary_search(&offset) {
            Ok(idx) => (idx as u32, self.starts[idx]),
            Err(idx) => ((idx - 1) as u32, self.starts[idx - 1]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Decoded {
        line: u32,
        col: u32,
        len: u32,
        ty: u32,
    }

    fn decode(tokens: &[SemanticToken]) -> Vec<Decoded> {
        let mut line = 0;
        let mut col = 0;
        let mut out = Vec::new();
        for t in tokens {
            if t.delta_line != 0 {
                line += t.delta_line;
                col = t.delta_start;
            } else {
                col += t.delta_start;
            }
            out.push(Decoded {
                line,
                col,
                len: t.length,
                ty: t.token_type,
            });
        }
        out
    }

    fn at<'a>(d: &'a [Decoded], line: u32, col: u32) -> &'a Decoded {
        d.iter()
            .find(|t| t.line == line && t.col == col)
            .unwrap_or_else(|| panic!("no token at {line}:{col}"))
    }

    #[test]
    fn legend_indices_match_constants() {
        assert_eq!(TOKEN_TYPES[TYPE as usize], SemanticTokenType::TYPE);
        assert_eq!(TOKEN_TYPES[ENUM_MEMBER as usize], SemanticTokenType::ENUM_MEMBER);
        assert_eq!(TOKEN_TYPES[COMMENT as usize], SemanticTokenType::COMMENT);
    }

    #[test]
    fn simple_struct_tokens() {
        let src = "struct Packet { len: u8, data: [u8; len] }";
        let d = decode(&compute(src));

        // "struct" keyword at col 0
        let kw = at(&d, 0, 0);
        assert_eq!(kw.ty, KEYWORD);
        assert_eq!(kw.len, 6);
        // "Packet" -> TYPE (declaration) at col 7
        assert_eq!(at(&d, 0, 7).ty, TYPE);
        // "len" field -> PROPERTY at col 16
        assert_eq!(at(&d, 0, 16).ty, PROPERTY);
        // "u8" -> TYPE at col 21
        assert_eq!(at(&d, 0, 21).ty, TYPE);
        // "data" field -> PROPERTY at col 25
        assert_eq!(at(&d, 0, 25).ty, PROPERTY);
        // "len" reference inside [u8; len] -> VARIABLE
        assert!(d.iter().any(|t| t.ty == VARIABLE));
    }

    #[test]
    fn comment_and_number_and_decorator() {
        let src = "// hi\n@endian(big) struct S { v = x1f }";
        let d = decode(&compute(src));
        // line comment
        let c = at(&d, 0, 0);
        assert_eq!(c.ty, COMMENT);
        // @endian decorator name "endian" at line 1 col 1
        assert_eq!(at(&d, 1, 1).ty, DECORATOR);
        // numeric literal x1f -> NUMBER spanning the whole token incl. `x` prefix
        let num = at(&d, 1, 28);
        assert_eq!(num.ty, NUMBER);
        assert_eq!(num.len, 3);
    }

    #[test]
    fn parse_error_still_yields_lexical_tokens() {
        // missing field value -> parse fails, but keywords/comments still scan
        let src = "// note\nstruct Broken { x: }";
        let d = decode(&compute(src));
        assert!(d.iter().any(|t| t.ty == COMMENT));
        assert!(d.iter().any(|t| t.ty == KEYWORD));
    }

    #[test]
    fn rich_spec_covers_visitor_paths_without_panicking() {
        let src = "@endian(big)\n\
                   struct Packet {\n\
                       version: b<4>,\n\
                       @skip ihl: b<4>,\n\
                       total_len: u16,\n\
                       flags: u8,\n\
                       if (flags == 1) {\n\
                           opt: u16,\n\
                       }\n\
                       @hook(parse_name, NameRef) name: [u8],\n\
                       tail: concat(u8, @skip u8),\n\
                       body: union(version) {\n\
                           4 => V4 { addr: u32 },\n\
                           _ => Other {},\n\
                       },\n\
                   }";
        let d = decode(&compute(src));
        for kind in [TYPE, PROPERTY, KEYWORD, DECORATOR, NUMBER, OPERATOR] {
            assert!(d.iter().any(|t| t.ty == kind), "missing token kind {kind}");
        }
        // V4 inline-struct name is a TYPE; addr is a PROPERTY
        assert!(d.iter().any(|t| t.ty == TYPE));
    }

    #[test]
    fn tokens_are_sorted_and_non_overlapping() {
        let src = "@endian(big)\nstruct Multi {\n  a: u8,\n  b: union(a) { 1 => X { y: u16 }, _ => Y {} }\n}";
        let tokens = compute(src);
        let d = decode(&tokens);
        for w in d.windows(2) {
            let prev_end = if w[0].line == w[1].line {
                w[0].col + w[0].len
            } else {
                0
            };
            if w[0].line == w[1].line {
                assert!(w[1].col >= prev_end, "overlap at {}:{}", w[1].line, w[1].col);
            }
        }
    }
}
