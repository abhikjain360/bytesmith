use std::collections::{HashMap, HashSet};

use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::quote;

use struct_::*;

mod struct_;

pub struct CodeGen<'a> {
    todo: HashMap<&'a str, Todo<'a>>,
    done: HashMap<&'a str, GeneratedStruct>,
    error_enum: Option<TokenStream>,
}

pub struct Todo<'a> {
    origin: &'a ast::Struct<'a>,
    dependencies: HashSet<&'a str>,
    dependants: HashSet<&'a str>,
}

impl<'a> Todo<'a> {
    pub fn new(origin: &'a ast::Struct<'a>) -> Self {
        Self {
            origin,
            dependencies: HashSet::new(),
            dependants: HashSet::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("duplicate struct definition '{name}'")]
    DuplicateStruct { name: String },
    #[error("failed to generate struct '{name}'")]
    GenerateStruct {
        name: String,
        #[source]
        error: struct_::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Len {
    byte: usize,
    bit: usize,
}

impl std::ops::Add for Len {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut byte = self.byte + other.byte;
        let mut bit = self.bit + other.bit;
        byte += bit / 8;
        bit %= 8;
        Self { byte, bit }
    }
}

impl<'a> CodeGen<'a> {
    pub fn generate(ast: &'a [ast::Definition<'a>]) -> Result<String, Error> {
        let mut structs = HashMap::new();
        let mut reverse_deps = HashMap::<_, HashSet<_>>::new();

        let mut roots = Vec::new();
        let mut error_enum = None;

        for def in ast {
            let s = match def {
                ast::Definition::Struct(s) => s,
                ast::Definition::Error(variants) => {
                    error_enum = Some(variants);
                    continue;
                }
            };

            let mut new_s = Todo::new(s);
            find_dependencies(&s.items, &mut new_s.dependencies);

            if new_s.dependencies.is_empty() {
                roots.push(s.name);
            } else {
                for dependency in &new_s.dependencies {
                    reverse_deps.entry(*dependency).or_default().insert(s.name);
                }
            }

            if structs.insert(s.name, new_s).is_some() {
                return Err(Error::DuplicateStruct {
                    name: s.name.to_owned(),
                });
            }
        }

        for (struct_, actual) in reverse_deps {
            let dependants = &mut structs.get_mut(struct_).expect("dependant not found when they should have been added when building dependencies").dependants;
            *dependants = actual;
        }

        let mut me = Self {
            todo: structs,
            done: HashMap::new(),
            error_enum: error_enum.map(|variants| generate_error_enum(&variants)),
        };
        let mut next = Vec::new();
        while !roots.is_empty() {
            for root in roots.drain(..) {
                let todo = me.todo.remove(root).expect("root not found in todo");

                // Updated: Clone the origin struct because StructCtx now takes ownership
                let generated = StructCtx::new(todo.origin.clone(), &me.done)
                    .generate()
                    .map_err(|e| Error::GenerateStruct {
                        name: root.to_string(),
                        error: e,
                    })?;
                me.done.insert(root, generated);

                for dependant in todo.dependants {
                    if let Some(dep_todo) = me.todo.get(dependant) {
                        if dep_todo
                            .dependencies
                            .iter()
                            .all(|d| me.done.contains_key(d))
                        {
                            if !next.contains(&dependant) {
                                next.push(dependant);
                            }
                        }
                    }
                }
            }

            std::mem::swap(&mut next, &mut roots);
        }

        me.print()
    }

    fn print(self) -> Result<String, Error> {
        let mut combined: TokenStream = self.done.into_values().map(|s| s.tokens).collect();
        if let Some(error_tokens) = self.error_enum {
            combined.extend(error_tokens);
        }
        let file: syn::File = syn::parse2(combined).expect("failed to parse generated tokens");
        Ok(prettyplease::unparse(&file))
    }
}

fn generate_error_enum(variants: &[ast::ErrorVariant]) -> TokenStream {
    let mut variant_tokens = TokenStream::new();
    for v in variants {
        let name = proc_macro2::Ident::new_raw(v.name, proc_macro2::Span::call_site());
        if v.fields.is_empty() {
            variant_tokens.extend(quote! { #name, });
        } else {
            let mut inner_fields = TokenStream::new();
            for (fname, ftype) in &v.fields {
                let fname_ident =
                    proc_macro2::Ident::new_raw(fname, proc_macro2::Span::call_site());
                let (_, type_token, _) = match_primitive(ftype);
                inner_fields.extend(quote! { #fname_ident: #type_token, });
            }
            variant_tokens.extend(quote! { #name { #inner_fields }, });
        }
    }

    quote! {
        #[derive(Debug, Clone, PartialEq)]
        pub enum Error {
            UnexpectedEof,
            BadLength,
            InvalidValue,
            #variant_tokens
        }
    }
}

fn find_dependencies<'a>(
    struct_items: &[ast::StructItem<'a>],
    dependencies: &mut HashSet<&'a str>,
) {
    for item in struct_items {
        match item {
            ast::StructItem::Field(ast::Field {
                value: ast::FieldValue::Type(ty),
                ..
            }) => find_type_dependencies(ty, dependencies),
            ast::StructItem::Conditional(conditional) => {
                find_conditional_dependencies(conditional, dependencies)
            }
            _ => {}
        }
    }
}

fn find_type_dependencies<'a>(ty: &ast::Type<'a>, dependencies: &mut HashSet<&'a str>) {
    match ty {
        ast::Type::StructRef(path) => {
            dependencies.insert(path.last().unwrap());
        }
        ast::Type::Array(array_ty) => {
            find_type_dependencies(&array_ty.elem_ty, dependencies);
        }
        ast::Type::Concat(fields) => {
            for field in fields {
                if let ast::FieldValue::Type(ty) = &field.value {
                    find_type_dependencies(ty, dependencies);
                }
            }
        }
        ast::Type::Union(union) => {
            for variant in &union.variants {
                if let ast::UnionBody::NamedInline(_, items) = &variant.body {
                    find_dependencies(items, dependencies);
                }
            }
        }
        _ => {}
    }
}

fn find_conditional_dependencies<'a>(
    conditional: &ast::Conditional<'a>,
    dependencies: &mut HashSet<&'a str>,
) {
    find_dependencies(&conditional.then_branch, dependencies);
    if let Some(else_branch) = &conditional.else_branch {
        find_dependencies(else_branch, dependencies);
    }
}

pub(crate) fn match_primitive(primitive: &ast::Primitive) -> (Len, TokenStream, bool) {
    match primitive {
        ast::Primitive::U8 => (Len { byte: 1, bit: 0 }, quote! { u8 }, true),
        ast::Primitive::U16 => (Len { byte: 2, bit: 0 }, quote! { u16 }, true),
        ast::Primitive::U32 => (Len { byte: 4, bit: 0 }, quote! { u32 }, true),
        ast::Primitive::U64 => (Len { byte: 8, bit: 0 }, quote! { u64 }, true),
        ast::Primitive::U128 => (Len { byte: 16, bit: 0 }, quote! { u128 }, true),
        ast::Primitive::BitField(width) => {
            let width = *width as usize;
            let ty = if width <= 8 {
                quote! { u8 }
            } else if width <= 16 {
                quote! { u16 }
            } else if width <= 32 {
                quote! { u32 }
            } else if width <= 64 {
                quote! { u64 }
            } else {
                quote! { u128 }
            };

            (
                Len {
                    byte: width / 8,
                    bit: width % 8,
                },
                ty,
                false,
            )
        }
    }
}

#[derive(Clone, Copy)]
pub enum ExprContext {
    Parse,
    Accessor,
}

pub(crate) fn generate_variable(
    path: &[&str],
    receiver: &TokenStream,
    context: ExprContext,
) -> TokenStream {
    match context {
        ExprContext::Parse => {
            let mut tokens = TokenStream::new();
            for (i, segment) in path.iter().enumerate() {
                let ident = proc_macro2::Ident::new_raw(segment, proc_macro2::Span::call_site());
                if i == 0 {
                    tokens = quote! { #ident };
                } else {
                    tokens = quote! { #tokens.#ident().0 };
                }
            }
            tokens
        }
        ExprContext::Accessor => {
            let mut tokens = quote! { #receiver };
            for segment in path {
                let ident = proc_macro2::Ident::new_raw(segment, proc_macro2::Span::call_site());
                tokens = quote! { #tokens.#ident().0 };
            }
            tokens
        }
    }
}

pub(crate) fn generate_literal(lit: &ast::NumericLiteral) -> TokenStream {
    match lit {
        ast::NumericLiteral::Decimal(v) => {
            let lit = proc_macro2::Literal::u128_unsuffixed(*v);
            quote! { #lit }
        }
        ast::NumericLiteral::Hex { value, .. } => {
            let lit = proc_macro2::Literal::u128_unsuffixed(*value);
            quote! { #lit }
        }
        ast::NumericLiteral::Binary { value, .. } => {
            let lit = proc_macro2::Literal::u128_unsuffixed(*value);
            quote! { #lit }
        }
    }
}

pub(crate) fn generate_math_expr(
    expr: &ast::MathExpr,
    receiver: &TokenStream,
    context: ExprContext,
) -> TokenStream {
    match expr {
        ast::MathExpr::Atom(atom) => match atom {
            ast::NumericAtom::Literal(lit) => generate_literal(lit),
            ast::NumericAtom::Variable(path) => generate_variable(path, receiver, context),
        },
        ast::MathExpr::Binary(lhs, op, rhs) => {
            let lhs = generate_math_expr(lhs, receiver, context);
            let rhs = generate_math_expr(rhs, receiver, context);
            match op {
                ast::MathOp::Add => quote! { (#lhs + #rhs) },
                ast::MathOp::Sub => quote! { (#lhs - #rhs) },
                ast::MathOp::Mul => quote! { (#lhs * #rhs) },
                ast::MathOp::Div => quote! { (#lhs / #rhs) },
                ast::MathOp::Mod => quote! { (#lhs % #rhs) },
                ast::MathOp::BitAnd => quote! { (#lhs & #rhs) },
                ast::MathOp::BitOr => quote! { (#lhs | #rhs) },
                ast::MathOp::BitXor => quote! { (#lhs ^ #rhs) },
            }
        }
    }
}

pub(crate) fn generate_bool_expr(
    expr: &ast::BoolExpr,
    receiver: &TokenStream,
    context: ExprContext,
) -> TokenStream {
    match expr {
        ast::BoolExpr::Comparison(lhs, op, rhs) => {
            let lhs = generate_math_expr(lhs, receiver, context);
            let rhs = generate_math_expr(rhs, receiver, context);
            match op {
                ast::CmpOp::Eq => quote! { (#lhs == #rhs) },
                ast::CmpOp::Neq => quote! { (#lhs != #rhs) },
                ast::CmpOp::Lt => quote! { (#lhs < #rhs) },
                ast::CmpOp::Gt => quote! { (#lhs > #rhs) },
                ast::CmpOp::Le => quote! { (#lhs <= #rhs) },
                ast::CmpOp::Ge => quote! { (#lhs >= #rhs) },
            }
        }
        ast::BoolExpr::Logic(lhs, op, rhs) => {
            let lhs = generate_bool_expr(lhs, receiver, context);
            let rhs = generate_bool_expr(rhs, receiver, context);
            match op {
                ast::LogicOp::And => quote! { (#lhs && #rhs) },
                ast::LogicOp::Or => quote! { (#lhs || #rhs) },
            }
        }
        ast::BoolExpr::Not(inner) => {
            let inner = generate_bool_expr(inner, receiver, context);
            quote! { !(#inner) }
        }
    }
}
