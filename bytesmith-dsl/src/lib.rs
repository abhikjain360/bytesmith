use std::ops::Range;

/// A byte span into the DSL source: the half-open range `[start, end)`.
///
/// Spans deliberately do **not** participate in equality: `PartialEq` always
/// returns `true`. Two AST nodes therefore compare equal iff their structure
/// matches, regardless of where in the source they were parsed from. This keeps
/// value-based comparisons and tests span-independent.
#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const DUMMY: Span = Span { start: 0, end: 0 };

    pub fn new(start: u32, end: u32) -> Span {
        Span { start, end }
    }
}

impl PartialEq for Span {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Span {}

impl From<Range<usize>> for Span {
    fn from(r: Range<usize>) -> Span {
        Span {
            start: r.start as u32,
            end: r.end as u32,
        }
    }
}

/// An identifier token paired with its source span.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ident<'a> {
    pub text: &'a str,
    pub span: Span,
}

impl<'a> Ident<'a> {
    pub fn new(text: &'a str, span: Span) -> Ident<'a> {
        Ident { text, span }
    }
}

impl PartialEq<&str> for Ident<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

impl PartialEq<str> for Ident<'_> {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

impl<'a> From<&'a str> for Ident<'a> {
    fn from(text: &'a str) -> Ident<'a> {
        Ident {
            text,
            span: Span::DUMMY,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoolBinaryOp {
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumericBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Bool(BoolBinaryOp),
    Numeric(NumericBinaryOp),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntLiteral {
    pub value: usize,
    pub width: u8,
    pub ty: IntType,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntType {
    Decimal,
    Hex,
    Binary,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal<'a> {
    Int(IntLiteral),
    String(&'a str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr<'a> {
    pub kind: ExprKind<'a>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind<'a> {
    Literal(Literal<'a>),
    Path(Vec<Ident<'a>>),
    Binary(Box<BinaryExpr<'a>>),
    Call(Ident<'a>, Vec<Expr<'a>>), // macros
    Tuple(Vec<Expr<'a>>),           // tuple matching in unions
    RawType(&'a str),               // raw Rust type token, e.g. @hook return type `Vec<&'a [u8]>`
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr<'a> {
    pub lhs: Expr<'a>,
    pub op: BinaryOp,
    pub op_span: Span,
    pub rhs: Expr<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute<'a> {
    pub name: Ident<'a>,
    pub args: Vec<Expr<'a>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field<'a> {
    pub name: Ident<'a>,
    pub attributes: Vec<Attribute<'a>>,
    pub value: FieldValue<'a>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue<'a> {
    Type(Type<'a>),
    Constraint(Expr<'a>), // field = 0x10
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conditional<'a> {
    pub condition: Expr<'a>,
    pub then_branch: Vec<StructItem<'a>>,
    pub else_branch: Option<Vec<StructItem<'a>>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructItem<'a> {
    Field(Field<'a>),
    Conditional(Conditional<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnionMatcher<'a> {
    pub kind: UnionMatcherKind<'a>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnionMatcherKind<'a> {
    Literal(Literal<'a>),
    Wildcard,
    Tuple(Vec<UnionMatcher<'a>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedInlineStruct<'a> {
    pub name: Ident<'a>,
    pub attributes: Vec<Attribute<'a>>,
    pub items: Vec<StructItem<'a>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnionBody<'a> {
    // spec: "Echo { ... }".
    // effectively defining a struct inline
    NamedInline(NamedInlineStruct<'a>),
    // error variant: @error(ERROR_NAME { field: expr, ... })
    Error(Ident<'a>, Vec<(Ident<'a>, Expr<'a>)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnionVariant<'a> {
    pub matchers: Vec<UnionMatcher<'a>>,
    pub body: UnionBody<'a>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Union<'a> {
    pub args: Vec<Ident<'a>>, // union(arg1, arg2)
    pub variants: Vec<UnionVariant<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayElemType<'a> {
    pub kind: ArrayElemTypeKind<'a>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayElemTypeKind<'a> {
    BitField(u8),
    Primitive(Primitive),
    StructRef(Ident<'a>), // Reference to another struct
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayType<'a> {
    pub elem_ty: ArrayElemType<'a>,
    pub size: Option<Expr<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConcatItem<'a> {
    pub attributes: Vec<Attribute<'a>>,
    pub ty: Type<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type<'a> {
    pub kind: TypeKind<'a>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind<'a> {
    BitField(u8), // b<N>
    Primitive(Primitive),
    Array(ArrayType<'a>),
    StructRef(Ident<'a>),        // Reference to another struct
    Concat(Vec<ConcatItem<'a>>), // concat(@attr type, ...)
    Union(Union<'a>),            // union(...) { ... }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Struct<'a> {
    pub name: Ident<'a>,
    pub attributes: Vec<Attribute<'a>>,
    pub items: Vec<StructItem<'a>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorVariant<'a> {
    pub name: Ident<'a>,
    pub fields: Vec<(Ident<'a>, Primitive)>, // Spec says primitive fields only for errors
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Definition<'a> {
    Struct(Struct<'a>),
    Error(Vec<ErrorVariant<'a>>),
}
