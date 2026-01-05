use winnow::{
    Parser,
    ascii::{digit1, hex_digit1, multispace0},
    combinator::{
        alt, delimited, dispatch, fail, opt, peek, preceded, repeat, separated, seq, terminated,
    },
    token::{any, take_until, take_while},
};

use binparse_dsl as ast;

// --- Whitespace & Comments ---

fn line_comment<'a>(input: &mut &'a str) -> winnow::Result<()> {
    ("//", take_while(0.., |c| c != '\n'), opt('\n'))
        .void()
        .parse_next(input)
}

fn block_comment<'a>(input: &mut &'a str) -> winnow::Result<()> {
    ("/*", take_until(0.., "*/"), "*/").void().parse_next(input)
}

fn ws<'a>(input: &mut &'a str) -> winnow::Result<()> {
    loop {
        let start_len = input.len();
        multispace0.parse_next(input)?;
        if line_comment.parse_next(input).is_ok() {
            continue;
        }
        if block_comment.parse_next(input).is_ok() {
            continue;
        }
        if start_len == input.len() {
            break;
        }
    }
    Ok(())
}

fn padded<'a, O, F>(mut inner: F) -> impl Parser<&'a str, O, winnow::error::ContextError>
where
    F: Parser<&'a str, O, winnow::error::ContextError>,
{
    move |input: &mut &'a str| {
        ws.parse_next(input)?;
        let res = inner.parse_next(input)?;
        ws.parse_next(input)?;
        Ok(res)
    }
}

// --- Identifiers ---

fn ident_raw<'a>(input: &mut &'a str) -> winnow::Result<&'a str> {
    take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '_')
        .verify(|s: &str| {
            s.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        })
        .parse_next(input)
}

fn identifier<'a>(input: &mut &'a str) -> winnow::Result<&'a str> {
    let start = *input;
    let i = ident_raw(input)?;
    let reserved = ["struct", "union", "concat", "if", "else", "error", "match"];
    if reserved.contains(&i) {
        *input = start;
        fail.parse_next(input)
    } else {
        Ok(i)
    }
}

fn path<'a>(input: &mut &'a str) -> winnow::Result<Vec<&'a str>> {
    separated(1.., identifier, ".").parse_next(input)
}

// --- Literals ---

#[derive(Debug, thiserror::Error)]
pub enum IntLiteralError {
    #[error("width too large: {0}")]
    WidthTooLarge(#[from] std::num::TryFromIntError),
    #[error("invalid integer: {0}")]
    InvalidInt(#[from] std::num::ParseIntError),
}

fn numeric_literal<'a>(input: &mut &'a str) -> winnow::Result<ast::NumericLiteral> {
    dispatch! {peek(any);
        'x' => hex_literal,
        'b' => binary_literal,
        '0' => alt((hex_literal, binary_literal, decimal_literal)),
        '1'..='9' => decimal_literal,
        _ => fail
    }
    .parse_next(input)
}

fn decimal_literal<'a>(input: &mut &'a str) -> winnow::Result<ast::NumericLiteral> {
    digit1
        .try_map(|s: &str| {
            s.parse::<u128>()
                .map(ast::NumericLiteral::Decimal)
                .map_err(IntLiteralError::InvalidInt)
        })
        .parse_next(input)
}

fn hex_literal<'a>(input: &mut &'a str) -> winnow::Result<ast::NumericLiteral> {
    preceded("x", hex_digit1)
        .try_map(|s: &str| {
            let width = s.len().try_into()?;
            u128::from_str_radix(s, 16)
                .map(|value| ast::NumericLiteral::Hex { value, width })
                .map_err(IntLiteralError::InvalidInt)
        })
        .parse_next(input)
}

fn binary_literal<'a>(input: &mut &'a str) -> winnow::Result<ast::NumericLiteral> {
    preceded("b", take_while(1.., |c| c == '0' || c == '1'))
        .try_map(|s: &str| {
            let width = s.len().try_into()?;
            u128::from_str_radix(s, 2)
                .map(|value| ast::NumericLiteral::Binary { value, width })
                .map_err(IntLiteralError::InvalidInt)
        })
        .parse_next(input)
}

fn string_literal<'a>(input: &mut &'a str) -> winnow::Result<&'a str> {
    delimited('"', take_while(0.., |c| c != '"'), '"').parse_next(input)
}

// --- Math Expressions ---

fn numeric_atom<'a>(input: &mut &'a str) -> winnow::Result<ast::NumericAtom<'a>> {
    alt((
        numeric_literal.map(ast::NumericAtom::Literal),
        path.map(ast::NumericAtom::Variable),
    ))
    .parse_next(input)
}

fn math_atom<'a>(input: &mut &'a str) -> winnow::Result<ast::MathExpr<'a>> {
    alt((
        delimited(padded('('), math_expr, padded(')')),
        numeric_atom.map(ast::MathExpr::Atom),
    ))
    .parse_next(input)
}

fn math_product<'a>(input: &mut &'a str) -> winnow::Result<ast::MathExpr<'a>> {
    let mut lhs = math_atom(input)?;
    loop {
        let start = *input;
        let op_res: winnow::Result<ast::MathOp> = padded(alt((
            "*".map(|_| ast::MathOp::Mul),
            "/".map(|_| ast::MathOp::Div),
            "%".map(|_| ast::MathOp::Mod),
        )))
        .parse_next(input);

        match op_res {
            Ok(op) => {
                let rhs = math_atom(input)?;
                lhs = ast::MathExpr::Binary(Box::new(lhs), op, Box::new(rhs));
            }
            Err(_) => {
                *input = start;
                break;
            }
        }
    }
    Ok(lhs)
}

fn math_sum<'a>(input: &mut &'a str) -> winnow::Result<ast::MathExpr<'a>> {
    let mut lhs = math_product(input)?;
    loop {
        let start = *input;
        let op_res: winnow::Result<ast::MathOp> = padded(alt((
            "+".map(|_| ast::MathOp::Add),
            "-".map(|_| ast::MathOp::Sub),
        )))
        .parse_next(input);

        match op_res {
            Ok(op) => {
                let rhs = math_product(input)?;
                lhs = ast::MathExpr::Binary(Box::new(lhs), op, Box::new(rhs));
            }
            Err(_) => {
                *input = start;
                break;
            }
        }
    }
    Ok(lhs)
}

fn math_expr<'a>(input: &mut &'a str) -> winnow::Result<ast::MathExpr<'a>> {
    // Bitwise ops (lower precedence than sum)
    let mut lhs = math_sum(input)?;
    loop {
        let start = *input;
        let op_res: winnow::Result<ast::MathOp> = padded(alt((
            "&".verify(|s: &str| !s.starts_with("&&"))
                .map(|_| ast::MathOp::BitAnd),
            "^".map(|_| ast::MathOp::BitXor),
            "|".verify(|s: &str| !s.starts_with("||"))
                .map(|_| ast::MathOp::BitOr),
        )))
        .parse_next(input);

        match op_res {
            Ok(op) => {
                let rhs = math_sum(input)?;
                lhs = ast::MathExpr::Binary(Box::new(lhs), op, Box::new(rhs));
            }
            Err(_) => {
                *input = start;
                break;
            }
        }
    }
    Ok(lhs)
}

// --- Boolean Expressions ---

fn comparison<'a>(input: &mut &'a str) -> winnow::Result<ast::BoolExpr<'a>> {
    // We expect lhs and rhs to be math expressions
    let lhs = math_expr(input)?;
    let op = padded(alt((
        "==".map(|_| ast::CmpOp::Eq),
        "!=".map(|_| ast::CmpOp::Neq),
        "<=".map(|_| ast::CmpOp::Le),
        ">=".map(|_| ast::CmpOp::Ge),
        "<".map(|_| ast::CmpOp::Lt),
        ">".map(|_| ast::CmpOp::Gt),
    )))
    .parse_next(input)?;
    let rhs = math_expr(input)?;
    Ok(ast::BoolExpr::Comparison(lhs, op, rhs))
}

fn bool_term<'a>(input: &mut &'a str) -> winnow::Result<ast::BoolExpr<'a>> {
    alt((
        delimited(padded('('), bool_expr, padded(')')),
        preceded(padded('!'), bool_term).map(|e| ast::BoolExpr::Not(Box::new(e))),
        comparison,
    ))
    .parse_next(input)
}

fn logic_and<'a>(input: &mut &'a str) -> winnow::Result<ast::BoolExpr<'a>> {
    let mut lhs = bool_term(input)?;
    loop {
        let start = *input;
        let op_res: winnow::Result<ast::LogicOp> =
            padded("&&".map(|_| ast::LogicOp::And)).parse_next(input);
        match op_res {
            Ok(op) => {
                let rhs = bool_term(input)?;
                lhs = ast::BoolExpr::Logic(Box::new(lhs), op, Box::new(rhs));
            }
            Err(_) => {
                *input = start;
                break;
            }
        }
    }
    Ok(lhs)
}

fn bool_expr<'a>(input: &mut &'a str) -> winnow::Result<ast::BoolExpr<'a>> {
    let mut lhs = logic_and(input)?;
    loop {
        let start = *input;
        let op_res: winnow::Result<ast::LogicOp> =
            padded("||".map(|_| ast::LogicOp::Or)).parse_next(input);
        match op_res {
            Ok(op) => {
                let rhs = logic_and(input)?;
                lhs = ast::BoolExpr::Logic(Box::new(lhs), op, Box::new(rhs));
            }
            Err(_) => {
                *input = start;
                break;
            }
        }
    }
    Ok(lhs)
}

// --- Patterns ---

fn pattern<'a>(input: &mut &'a str) -> winnow::Result<ast::Pattern> {
    alt((
        "_".map(|_| ast::Pattern::Wildcard),
        numeric_literal.map(ast::Pattern::Literal),
        pattern_tuple,
    ))
    .parse_next(input)
}

fn pattern_tuple<'a>(input: &mut &'a str) -> winnow::Result<ast::Pattern> {
    delimited(
        padded('('),
        separated(0.., pattern, padded(',')),
        padded(')'),
    )
    .map(ast::Pattern::Tuple)
    .parse_next(input)
}

// --- Types & Attributes ---

fn attribute_arg<'a>(input: &mut &'a str) -> winnow::Result<ast::AttributeArg<'a>> {
    alt((
        string_literal.map(ast::AttributeArg::String),
        // Try type parser (primitives, arrays, concat, union)
        // Note: type_parser includes StructRef which is a path.
        // We need to decide precedence.
        // For now, let's try type_parser (excluding StructRef?? no, include it)
        // If we want MathExpr for paths, we should put MathExpr before Type if Type is ambiguous?
        // But Type includes keywords like 'b<N>', 'union', 'concat'.
        // MathExpr includes literals and paths.
        // Let's try explicit matches first.
        bool_expr.map(ast::AttributeArg::Bool),
        // Note: bool_expr starts with math_expr which can be a path.
        // If I have "big", bool_expr (comparison) fails.
        // If I have "len", math_expr matches.
        // If I have "MyStruct", math_expr matches.
        // Type parser matches "MyStruct" too.
        // Let's try type_parser.
        type_parser.map(ast::AttributeArg::Type),
        math_expr.map(ast::AttributeArg::Math),
    ))
    .parse_next(input)
}

fn attribute<'a>(input: &mut &'a str) -> winnow::Result<ast::Attribute<'a>> {
    seq! {ast::Attribute {
        _: '@',
        name: identifier,
        args: opt(delimited(padded('('), separated(0.., attribute_arg, padded(',')), padded(')')))
            .map(|o| o.unwrap_or_default()),
    }}
    .parse_next(input)
}

fn attributes<'a>(input: &mut &'a str) -> winnow::Result<Vec<ast::Attribute<'a>>> {
    repeat(0.., padded(attribute)).parse_next(input)
}

fn primitive<'a>(input: &mut &'a str) -> winnow::Result<ast::Primitive> {
    dispatch! {peek(any);
        'u' => alt((
            "u8".map(|_| ast::Primitive::U8),
            "u16".map(|_| ast::Primitive::U16),
            "u32".map(|_| ast::Primitive::U32),
            "u64".map(|_| ast::Primitive::U64),
            "u128".map(|_| ast::Primitive::U128),
        )),
        'b' => ("b", delimited('<', digit1, '>')).try_map(|(_, w_str): (&str, &str)| {
            w_str.parse::<u8>()
        }).verify(|w| *w <= 128).map(ast::Primitive::BitField),
        _ => fail
    }
    .parse_next(input)
}

fn type_parser<'a>(input: &mut &'a str) -> winnow::Result<ast::Type<'a>> {
    alt((
        array_type,
        concat_type,
        union_type,
        primitive.map(ast::Type::Primitive),
        padded(path).map(ast::Type::StructRef),
    ))
    .parse_next(input)
}

fn array_type<'a>(input: &mut &'a str) -> winnow::Result<ast::Type<'a>> {
    delimited(
        padded('['),
        (type_parser, opt(preceded(padded(';'), math_expr))),
        padded(']'),
    )
    .map(|(elem_ty, size_expr)| ast::Type::Array(Box::new(ast::ArrayType { elem_ty, size_expr })))
    .parse_next(input)
}

fn field_value<'a>(input: &mut &'a str) -> winnow::Result<ast::FieldValue<'a>> {
    alt((
        preceded(padded(':'), type_parser).map(ast::FieldValue::Type),
        preceded(padded('='), numeric_literal).map(ast::FieldValue::Constraint),
    ))
    .parse_next(input)
}

fn concat_type<'a>(input: &mut &'a str) -> winnow::Result<ast::Type<'a>> {
    preceded(
        padded("concat"),
        delimited(padded('('), separated(0.., field, padded(',')), padded(')')),
    )
    .map(ast::Type::Concat)
    .parse_next(input)
}

fn error_body<'a>(input: &mut &'a str) -> winnow::Result<ast::UnionBody<'a>> {
    // @error(ERROR_NAME { field: atom, ... })
    let _ = padded("@error").parse_next(input)?;
    let _ = padded('(').parse_next(input)?;
    let name = padded(identifier).parse_next(input)?;
    let fields = opt(delimited(
        padded('{'),
        separated(
            0..,
            seq! { padded(identifier), _: padded(':'), numeric_atom },
            padded(','),
        ),
        padded('}'),
    ))
    .parse_next(input)?
    .unwrap_or_default();
    let _ = padded(')').parse_next(input)?;
    Ok(ast::UnionBody::Error(name, fields))
}

fn union_body<'a>(input: &mut &'a str) -> winnow::Result<ast::UnionBody<'a>> {
    alt((
        error_body,
        seq! {
            padded(identifier),
            delimited(padded('{'), struct_items, padded('}'))
        }
        .map(|(n, items)| ast::UnionBody::NamedInline(n, items)),
    ))
    .parse_next(input)
}

fn union_variant<'a>(input: &mut &'a str) -> winnow::Result<ast::UnionVariant<'a>> {
    seq! {ast::UnionVariant {
        matchers: separated(1.., pattern, padded('|')),
        _: padded("=>"),
        body: union_body,
    }}
    .parse_next(input)
}

fn union_type<'a>(input: &mut &'a str) -> winnow::Result<ast::Type<'a>> {
    preceded(
        padded("union"),
        seq! {
            delimited(padded('('), separated(0.., path, padded(',')), padded(')')),
            delimited(padded('{'), union_variants, padded('}'))
        },
    )
    .map(|(target, variants)| ast::Type::Union(ast::Union { target, variants }))
    .parse_next(input)
}

fn union_variants<'a>(input: &mut &'a str) -> winnow::Result<Vec<ast::UnionVariant<'a>>> {
    let mut variants = Vec::new();
    loop {
        let start = *input;
        match union_variant.parse_next(input) {
            Ok(v) => {
                variants.push(v);
                let _ = padded(',').parse_next(input);
            }
            Err(_) => {
                *input = start;
                break;
            }
        }
    }
    Ok(variants)
}

fn struct_item<'a>(input: &mut &'a str) -> winnow::Result<ast::StructItem<'a>> {
    alt((
        conditional.map(ast::StructItem::Conditional),
        field_with_opt_comma.map(ast::StructItem::Field),
    ))
    .parse_next(input)
}

fn struct_items<'a>(input: &mut &'a str) -> winnow::Result<Vec<ast::StructItem<'a>>> {
    repeat(0.., struct_item).parse_next(input)
}

fn conditional<'a>(input: &mut &'a str) -> winnow::Result<ast::Conditional<'a>> {
    seq! {ast::Conditional {
        _: padded("if"),
        condition: delimited(padded('('), bool_expr, padded(')')),
        then_branch: delimited(padded('{'), struct_items, padded('}')),
        else_branch: opt(preceded(padded("else"), delimited(padded('{'), struct_items, padded('}')))),
    }}.parse_next(input)
}

fn field<'a>(input: &mut &'a str) -> winnow::Result<ast::Field<'a>> {
    seq! {ast::Field {
        attributes: attributes,
        name: padded(identifier),
        value: field_value,
    }}
    .parse_next(input)
}

fn field_with_opt_comma<'a>(input: &mut &'a str) -> winnow::Result<ast::Field<'a>> {
    terminated(field, opt(padded(','))).parse_next(input)
}

fn struct_def<'a>(input: &mut &'a str) -> winnow::Result<ast::Definition<'a>> {
    seq! {ast::Struct {
        attributes: attributes,
        _: padded("struct"),
        name: padded(identifier),
        items: delimited(padded('{'), struct_items, padded('}')),
    }}
    .map(ast::Definition::Struct)
    .parse_next(input)
}

fn error_variant<'a>(input: &mut &'a str) -> winnow::Result<ast::ErrorVariant<'a>> {
    let name = padded(identifier).parse_next(input)?;
    let fields = opt(delimited(
        padded('{'),
        separated(
            0..,
            seq! { padded(identifier), _: padded(':'), padded(primitive) },
            padded(','),
        ),
        padded('}'),
    ))
    .parse_next(input)?
    .unwrap_or_default();
    let _ = opt(padded(',')).parse_next(input)?;
    Ok(ast::ErrorVariant { name, fields })
}

fn error_def<'a>(input: &mut &'a str) -> winnow::Result<ast::Definition<'a>> {
    preceded(
        padded("error"),
        delimited(padded('{'), repeat(0.., padded(error_variant)), padded('}')),
    )
    .map(ast::Definition::Error)
    .parse_next(input)
}

/// Parse a BinParse DSL source string into a list of definitions.
fn parse<'a>(input: &mut &'a str) -> winnow::Result<Vec<ast::Definition<'a>>> {
    repeat(0.., padded(alt((struct_def, error_def)))).parse_next(input)
}

/// Convenience function that takes an owned string and returns Result.
pub fn parse_str(input: &str) -> Result<Vec<ast::Definition<'_>>, String> {
    parse.parse(input).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_helper(src: &str) -> Vec<ast::Definition<'_>> {
        match parse.parse(src) {
            Ok(defs) => defs,
            Err(e) => {
                panic!("Parse errors: \n{}", e);
            }
        }
    }

    #[test]
    fn test_simple_struct() {
        let src = r#"
            struct Simple {
                a: u8,
                b: u16,
            }
        "#;
        let defs = parse_helper(src);
        assert_eq!(defs.len(), 1);
    }

    #[test]
    fn test_array_expr() {
        let src = r#"
            struct Arr {
                len: u8,
                data: [u8; len * 2],
                complex: [u8; (len * 2) - 4],
            }
        "#;
        parse_helper(src);
    }

    #[test]
    fn test_conditional_if() {
        let src = r#"
            struct Cond {
                x: u8,
                if (x == 1) {
                    y: u8,
                }
            }
        "#;
        parse_helper(src);
    }

    #[test]
    fn test_type_inference_bitfield() {
        let src = r#"
            struct Infer {
                res = b011,
            }
        "#;
        let defs = parse_helper(src);
        match &defs[0] {
            ast::Definition::Struct(s) => match &s.items[0] {
                ast::StructItem::Field(f) => {
                    assert_eq!(
                        f.value,
                        ast::FieldValue::Constraint(ast::NumericLiteral::Binary {
                            value: 3,
                            width: 3,
                        })
                    );
                }
                _ => panic!("Expected field"),
            },
            _ => panic!("Expected struct"),
        }
    }

    #[test]
    fn test_union_simple() {
        let src = r#"
            struct U {
                t: u8,
                body: union(t) {
                    1 => A { x: u8 },
                }
            }
        "#;
        parse_helper(src);
    }

    #[test]
    fn test_union_error_fallback() {
        let src = r#"
            struct Packet {
                packet_type: b<3>,
                variant: union(packet_type) {
                    b010 => Something { data: u8 },
                    _ => @error(MISSING_FLAG { found: packet_type, expected: b010 })
                }
            }
        "#;
        parse_helper(src);
    }
}
