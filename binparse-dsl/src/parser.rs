use chumsky::prelude::*;
use chumsky::input::Emitter;
use chumsky::span::SimpleSpan;
use crate::ast::*;

// Lexer/Parser helper types
type PError<'a> = extra::Err<Rich<'a, char>>;

pub fn parser<'a>() -> impl Parser<'a, &'a str, Vec<Definition>, PError<'a>> {
    // Comments
    let single_line = just("//").then(any().and_is(just('\n').not()).repeated()).then_ignore(just('\n').ignored().or(end()));
    let multi_line = just("/*").then(any().and_is(just("*/").not()).repeated()).then_ignore(just("*/"));
    
    let comment = single_line.or(multi_line).ignored();

    // Filler: whitespace and comments
    let filler = choice((
        comment,
        text::whitespace()
    )).repeated().boxed();

    // Helper parsers
    let ident = text::ascii::ident()
        .map(|s: &str| s.to_string())
        .padded_by(filler.clone());
        
    let ident_with_span = text::ascii::ident()
        .map_with(|s: &str, e| (s.to_string(), e.span()))
        .padded_by(filler.clone());
    
    // Literals
    let decimal = text::int(10).map(|s: &str| Literal::Int(s.parse().unwrap()));
    let hex = just("0x").or(just("x")).ignore_then(text::int(16))
        .map(|s: &str| Literal::Int(i128::from_str_radix(s, 16).unwrap()));
    let binary = just("0b").or(just("b")).ignore_then(text::int(2))
        .map(|s: &str| Literal::Int(i128::from_str_radix(s, 2).unwrap()));
    
    let literal = choice((
        hex,
        binary,
        decimal,
    )).map(Expr::Literal).padded_by(filler.clone());

    let expr = recursive(|expr| {
        let atom = choice((
            literal,
            ident.clone().map(Expr::Ident),
            expr.clone().delimited_by(just('(').padded_by(filler.clone()), just(')').padded_by(filler.clone())),
        ));

        let product = atom.clone()
            .foldl(choice((just("*").to(BinaryOp::Mul), just("/").to(BinaryOp::Div))).padded_by(filler.clone()).then(atom.clone()).repeated(), 
            |lhs, (op, rhs)| Expr::Binary(Box::new(lhs), op, Box::new(rhs)));
            
        let sum = product.clone()
            .foldl(choice((just("+").to(BinaryOp::Add), just("-").to(BinaryOp::Sub))).padded_by(filler.clone()).then(product.clone()).repeated(),
            |lhs, (op, rhs)| Expr::Binary(Box::new(lhs), op, Box::new(rhs)));
            
        let comparison = sum.clone()
            .foldl(choice((
                just("==").to(BinaryOp::Eq),
                just("!=").to(BinaryOp::Neq),
                just("<=").to(BinaryOp::Le),
                just(">=").to(BinaryOp::Ge),
                just("<").to(BinaryOp::Lt),
                just(">").to(BinaryOp::Gt)
            )).padded_by(filler.clone()).then(sum.clone()).repeated(),
            |lhs, (op, rhs)| Expr::Binary(Box::new(lhs), op, Box::new(rhs)));

        comparison
    });

    let attribute = just('@').padded_by(filler.clone())
        .ignore_then(ident.clone())
        .then(expr.clone().separated_by(just(',').padded_by(filler.clone())).collect().delimited_by(just('(').padded_by(filler.clone()), just(')').padded_by(filler.clone())).or_not())
        .map(|(name, args): (String, Option<Vec<Expr>>)| Attribute {
            name,
            args: args.unwrap_or_default(),
        });

    let attributes = attribute.repeated().collect::<Vec<_>>();

    let struct_items_block = recursive(|items| {
        let primitive = choice((
            just("u8").to(Primitive::U8),
            just("u16").to(Primitive::U16),
            just("u32").to(Primitive::U32),
            just("u64").to(Primitive::U64),
            just("u128").to(Primitive::U128),
            just("i8").to(Primitive::I8),
            just("i16").to(Primitive::I16),
            just("i32").to(Primitive::I32),
            just("i64").to(Primitive::I64),
            just("i128").to(Primitive::I128),
            just("b").ignore_then(text::int(10).delimited_by(just('<'), just('>'))).map(|s: &str| Primitive::BitField(s.parse().unwrap())),
        )).map(Type::Primitive).padded_by(filler.clone());

        let ty = recursive(|ty| {
            let array = ty.clone()
                .then_ignore(just(';').padded_by(filler.clone()))
                .then(expr.clone())
                .delimited_by(just('[').padded_by(filler.clone()), just(']').padded_by(filler.clone()))
                .map(|(t, len)| Type::Array(Box::new(t), len));

            let concat_field = ident.clone().then_ignore(just(':').padded_by(filler.clone()))
                .then(ty.clone()) 
                .then(attributes.clone()) 
                .map(|((name, ty), attrs)| Field {
                    name: Some(name),
                    ty,
                    attributes: attrs,
                    value_constraint: None
                });
                
            let concat = just("concat").padded_by(filler.clone()).ignore_then(
                concat_field.separated_by(just(',').padded_by(filler.clone())).collect().delimited_by(just('(').padded_by(filler.clone()), just(')').padded_by(filler.clone()))
            ).map(Type::Concat);

            let union_body = choice((
                // Named Inline: Echo { ... }
                ident.clone().then(items.clone().delimited_by(just('{').padded_by(filler.clone()), just('}').padded_by(filler.clone())))
                    .map(|(name, fields)| UnionBody::NamedInline(name, fields)),
                // Inline: { ... }
                items.clone().delimited_by(just('{').padded_by(filler.clone()), just('}').padded_by(filler.clone()))
                    .map(UnionBody::InlineStruct),
                // Type Ref: Ident
                ident.clone().map(UnionBody::TypeRef),
            ));

            let union_variant = expr.clone().separated_by(just('|').padded_by(filler.clone())).collect()
                .then_ignore(just("=>").padded_by(filler.clone()))
                .then(union_body)
                .map(|(matchers, body)| UnionVariant { matchers, body });

            let union_def = just("union").padded_by(filler.clone()).ignore_then(
                ident.clone().separated_by(just(',').padded_by(filler.clone())).collect().delimited_by(just('(').padded_by(filler.clone()), just(')').padded_by(filler.clone()))
            ).then(
                union_variant.separated_by(just(',').padded_by(filler.clone())).allow_trailing().collect().delimited_by(just('{').padded_by(filler.clone()), just('}').padded_by(filler.clone()))
            ).map(|(args, variants)| Type::Union(UnionDef { args, variants }));

            primitive.clone()
                .or(array)
                .or(concat)
                .or(union_def)
                .or(ident.clone().map(Type::StructRef))
        });

        let field_decl = attributes.clone()
            .then(ident_with_span.clone())
            .then(just(':').padded_by(filler.clone()).ignore_then(ty.clone()).or_not())
            .then(just('=').padded_by(filler.clone()).ignore_then(expr.clone()).or_not())
            .then_ignore(just(',').padded_by(filler.clone()).or_not())
            .validate(|(((attrs, (name, span)), ty), constraint): (((Vec<Attribute>, (String, SimpleSpan)), Option<Type>), Option<Expr>), _, emitter: &mut Emitter<Rich<'a, char>>| {
                 if ty.is_none() && constraint.is_none() {
                     emitter.emit(Rich::custom(span, "Field must have a type or a constraint"));
                     Field { name: Some(name), ty: Type::Primitive(Primitive::U8), attributes: attrs, value_constraint: None }
                 } else if let Some(t) = ty {
                     Field { name: Some(name), ty: t, attributes: attrs, value_constraint: constraint }
                 } else {
                     Field { name: Some(name), ty: Type::Primitive(Primitive::U8), attributes: attrs, value_constraint: constraint }
                 }
            })
            .map(StructItem::Field);

        let conditional = just("if").padded_by(filler.clone())
            .ignore_then(expr.clone().delimited_by(just('(').padded_by(filler.clone()), just(')').padded_by(filler.clone())))
            .then(items.clone().delimited_by(just('{').padded_by(filler.clone()), just('}').padded_by(filler.clone())))
            .then(just("else").padded_by(filler.clone()).ignore_then(items.clone().delimited_by(just('{').padded_by(filler.clone()), just('}').padded_by(filler.clone()))).or_not())
            .map(|((condition, then_branch), else_branch)| StructItem::Conditional(Conditional {
                condition,
                then_branch,
                else_branch
            }));

        choice((
            conditional,
            field_decl
        )).repeated().collect()
    });

    let struct_def = attributes.clone()
        .then_ignore(just("struct").padded_by(filler.clone()))
        .then(ident.clone())
        .then(struct_items_block.delimited_by(just('{').padded_by(filler.clone()), just('}').padded_by(filler.clone())))
        .map(|((attrs, name), items)| Definition::Struct(StructDef {
            name,
            attributes: attrs,
            items,
        }));
        
    let error_field_type = choice((
            just("u8").to(Primitive::U8),
            just("u16").to(Primitive::U16),
            just("u32").to(Primitive::U32),
            just("u64").to(Primitive::U64),
            just("u128").to(Primitive::U128),
            just("i8").to(Primitive::I8),
            just("i16").to(Primitive::I16),
            just("i32").to(Primitive::I32),
            just("i64").to(Primitive::I64),
            just("i128").to(Primitive::I128),
            just("b").ignore_then(text::int(10).delimited_by(just('<'), just('>'))).map(|s: &str| Primitive::BitField(s.parse().unwrap())),
        )).padded_by(filler.clone());

    let error_variant = ident.clone()
        .then(
             ident.clone().then_ignore(just(':').padded_by(filler.clone()))
                  .then(error_field_type)
                  .separated_by(just(',').padded_by(filler.clone())).collect()
                  .delimited_by(just('{').padded_by(filler.clone()), just('}').padded_by(filler.clone()))
                  .or_not()
        )
        .then_ignore(just(',').padded_by(filler.clone()).or_not())
        .map(|(name, fields): (String, Option<Vec<(String, Primitive)>>)| ErrorVariant {
            name,
            fields: fields.unwrap_or_default(),
        });

    let error_def = just("error").padded_by(filler.clone())
        .ignore_then(error_variant.repeated().collect().delimited_by(just('{').padded_by(filler.clone()), just('}').padded_by(filler.clone())))
        .map(Definition::Error);

    choice((
        struct_def,
        error_def
    )).padded_by(filler.clone()).repeated().collect()
}
