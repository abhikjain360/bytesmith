#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Primitive {
    U8,
    U16,
    U32,
    U64,
    U128,
    BitField(u8), // b<N>
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MathOp {
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
pub enum CmpOp {
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumericLiteral {
    Decimal(u128),
    Hex { value: u128, width: u8 },
    Binary { value: u128, width: u8 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumericAtom<'a> {
    Literal(NumericLiteral),
    Variable(Vec<&'a str>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MathExpr<'a> {
    Atom(NumericAtom<'a>),
    Binary(Box<MathExpr<'a>>, MathOp, Box<MathExpr<'a>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoolExpr<'a> {
    Comparison(MathExpr<'a>, CmpOp, MathExpr<'a>),
    Logic(Box<BoolExpr<'a>>, LogicOp, Box<BoolExpr<'a>>),
    Not(Box<BoolExpr<'a>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttributeArg<'a> {
    String(&'a str),
    Type(Type<'a>),
    Math(MathExpr<'a>),
    Bool(BoolExpr<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute<'a> {
    pub name: &'a str,
    pub args: Vec<AttributeArg<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub attributes: Vec<Attribute<'a>>,
    pub value: FieldValue<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue<'a> {
    Type(Type<'a>),
    Constraint(NumericLiteral), // field = 0x10
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conditional<'a> {
    pub condition: BoolExpr<'a>,
    pub then_branch: Vec<StructItem<'a>>,
    pub else_branch: Option<Vec<StructItem<'a>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StructItem<'a> {
    Field(Field<'a>),
    Conditional(Conditional<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Literal(NumericLiteral),
    Wildcard,
    Tuple(Vec<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnionVariant<'a> {
    pub matchers: Vec<Pattern>,
    pub body: UnionBody<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnionBody<'a> {
    NamedInline(&'a str, Vec<StructItem<'a>>),
    Error(&'a str, Vec<(&'a str, NumericAtom<'a>)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Union<'a> {
    pub target: Vec<Vec<&'a str>>, // union(a, b.c)
    pub variants: Vec<UnionVariant<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayType<'a> {
    pub elem_ty: Type<'a>,
    pub size_expr: Option<MathExpr<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type<'a> {
    Primitive(Primitive),
    Array(Box<ArrayType<'a>>),
    StructRef(Vec<&'a str>), // Reference to another struct
    Concat(Vec<Field<'a>>),  // concat(f1: type, ...)
    Union(Union<'a>),        // union(...) { ... }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Struct<'a> {
    pub name: &'a str,
    pub attributes: Vec<Attribute<'a>>,
    pub items: Vec<StructItem<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErrorVariant<'a> {
    pub name: &'a str,
    pub fields: Vec<(&'a str, Primitive)>, // Spec says primitive fields only for errors
}

#[derive(Debug, Clone, PartialEq)]
pub enum Definition<'a> {
    Struct(Struct<'a>),
    Error(Vec<ErrorVariant<'a>>),
}
