use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{GeneratedLen, attr::{Inherited, ParsedAttrs}, expr, field};

#[derive(Clone, Copy)]
pub(crate) enum DoneFieldType {
    Primitive,
    BitField,
    Other,
}

pub(crate) struct DoneField {
    pub(crate) name: String,
    pub(crate) field_type: DoneFieldType,
    pub(crate) conditional: bool,
}

pub(crate) struct StructAccum {
    pub(crate) name: syn::Ident,
    pub(crate) inherited: Inherited,
    pub(crate) offset: GeneratedLen,
    pub(crate) done_fields: Vec<DoneField>,
    pub(crate) other_entities: TokenStream,
    pub(crate) field_definitions: TokenStream,
    pub(crate) functions: TokenStream,
    pub(crate) parse_checks: TokenStream,
    pub(crate) last_offset_getter_fn_name: Option<syn::Ident>,
    pub(crate) condition: Option<syn::Ident>,
    pub(crate) conditional_count: usize,
}

pub(crate) struct GeneratedStruct {
    pub(crate) len: GeneratedLen,
    pub(crate) tokens: TokenStream,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to generate field '{name}': {error}")]
    Field {
        name: String,
        #[source]
        error: crate::field::Error,
    },
    #[error("'field {field}' needs byte-alignment, but previous fields didn't align")]
    Unaligned { field: String },
    #[error(transparent)]
    Attr(#[from] crate::attr::Error),
    #[error(transparent)]
    Expr(#[from] expr::Error),
}

impl StructAccum {
    pub(crate) fn new(name: &str, inherited: Inherited) -> Self {
        Self {
            name: format_ident!("{}", name),
            inherited,
            offset: GeneratedLen::Fixed(Len { byte: 0, bit: 0 }),
            done_fields: vec![],
            other_entities: TokenStream::new(),
            field_definitions: TokenStream::new(),
            functions: TokenStream::new(),
            parse_checks: TokenStream::new(),
            last_offset_getter_fn_name: None,
            condition: None,
            conditional_count: 0,
        }
    }
}

pub(crate) fn generate<'a>(
    ast: &'a ast::Struct<'a>,
    done: &mut HashMap<&'a str, GeneratedStruct>,
) -> Result<(), Error> {
    let attrs = ParsedAttrs::parse(&ast.attributes)?;
    let struct_inherited = attrs.merge_inherited(Inherited::default());
    let mut accum = StructAccum::new(ast.name, struct_inherited);

    generate_items(&ast.items, done, &mut accum)?;

    let StructAccum {
        name,
        inherited: _,
        offset,
        done_fields: _,
        other_entities,
        field_definitions,
        functions,
        parse_checks,
        last_offset_getter_fn_name,
        condition: _,
        conditional_count: _,
    } = accum;

    let parse_fn = if let Some(fn_name) = last_offset_getter_fn_name {
        quote! {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                #parse_checks
                let len = me.#fn_name();
                if len.bit != 0 {
                    return Err(binparse::ParseError::UnalignedLength(len));
                }
                if data.len() < len.byte {
                    return Err(binparse::ParseError::NotEnoughData { expected: len.byte, got: data.len() });
                }
                Ok((me, &data[len.byte..]))
            }
        }
    } else {
        quote! {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                Ok((Self { data }, data))
            }
        }
    };

    let tokens = quote! {
        #other_entities

        pub struct #name<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
            #field_definitions
        }

        impl<'a> #name<'a> {
            #parse_fn

            #functions
        }
    };

    done.insert(
        ast.name,
        GeneratedStruct {
            len: offset,
            tokens,
        },
    );

    Ok(())
}

fn generate_items<'a>(
    items: &'a [ast::StructItem<'a>],
    done: &HashMap<&'a str, GeneratedStruct>,
    accum: &mut StructAccum,
) -> Result<(), Error> {
    for item in items {
        match item {
            ast::StructItem::Field(ast_field) => {
                field::generate(ast_field, done, accum).map_err(|error| Error::Field {
                    name: ast_field.name.to_string(),
                    error,
                })?;
            }
            ast::StructItem::Conditional(conditional) => {
                generate_conditional(conditional, done, accum)?;
            }
        }
    }

    Ok(())
}

fn generate_conditional<'a>(
    conditional: &'a ast::Conditional<'a>,
    done: &HashMap<&'a str, GeneratedStruct>,
    accum: &mut StructAccum,
) -> Result<(), Error> {
    let condition = expr::lower(
        &conditional.condition,
        expr::ExprType::Bool,
        &accum.done_fields,
    )?
    .tokens;

    let index = accum.conditional_count;
    accum.conditional_count += 1;
    let present_fn_name = format_ident!("conditional_{}_present", index);
    let absent_fn_name = format_ident!("conditional_{}_absent", index);
    let end_offset_fn_name = format_ident!("conditional_{}_end_offset", index);

    let start_offset = accum.offset.clone();
    let start_getter_fn_name = accum.last_offset_getter_fn_name.clone();
    let start_expr = match &start_getter_fn_name {
        Some(fn_name) => quote! { self.#fn_name() },
        None => quote! { binparse::Len::ZERO },
    };

    let outer_gate = accum.condition.clone();
    let gated = |branch_condition: TokenStream| match &outer_gate {
        Some(gate) => quote! { self.#gate() && #branch_condition },
        None => branch_condition,
    };

    let then_condition = gated(quote! { #condition });
    accum.functions.extend(quote! {
        #[allow(dead_code, unused_parens)]
        fn #present_fn_name(&self) -> bool {
            #then_condition
        }
    });
    accum.condition = Some(present_fn_name.clone());
    generate_items(&conditional.then_branch, done, accum)?;
    let then_end_expr = match &accum.last_offset_getter_fn_name {
        Some(fn_name) => quote! { self.#fn_name() },
        None => quote! { binparse::Len::ZERO },
    };

    accum.offset = start_offset;
    accum.last_offset_getter_fn_name = start_getter_fn_name;

    let else_end_expr = match &conditional.else_branch {
        Some(else_branch) => {
            let else_condition = gated(quote! { !#condition });
            accum.functions.extend(quote! {
                #[allow(dead_code, unused_parens)]
                fn #absent_fn_name(&self) -> bool {
                    #else_condition
                }
            });
            accum.condition = Some(absent_fn_name);
            generate_items(else_branch, done, accum)?;
            match &accum.last_offset_getter_fn_name {
                Some(fn_name) => quote! { self.#fn_name() },
                None => quote! { binparse::Len::ZERO },
            }
        }
        None => start_expr,
    };

    accum.condition = outer_gate;

    accum.functions.extend(quote! {
        fn #end_offset_fn_name(&self) -> binparse::Len {
            if self.#present_fn_name() {
                #then_end_expr
            } else {
                #else_end_expr
            }
        }
    });

    accum.offset = GeneratedLen::Dynamic(quote! { self.#end_offset_fn_name() });
    accum.last_offset_getter_fn_name = Some(end_offset_fn_name);

    Ok(())
}
