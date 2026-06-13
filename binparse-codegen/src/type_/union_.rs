use std::collections::HashMap;
use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    GeneratedLen,
    attr::Inherited,
    expr,
    field::{self, FieldAccum},
    struct_::{DoneFieldType, GeneratedStruct, StructAccum},
    type_::{self, GeneratedTypeInfo},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("union has no arguments")]
    NoArguments,
    #[error("union has no variants")]
    NoVariants,
    #[error("matcher has {got} elements but union has {expected} arguments")]
    MatcherCountMismatch { expected: usize, got: usize },
}

pub(crate) fn generate(
    union: &ast::Union<'_>,
    done: &HashMap<&str, GeneratedStruct>,
    struct_accum: &mut StructAccum,
    accum: &mut FieldAccum,
    start_offset: GeneratedLen,
    inherited: Inherited,
) -> Result<GeneratedTypeInfo, type_::Error> {
    let num_args = union.args.len();
    if num_args == 0 {
        return Err(type_::Error::Union(Error::NoArguments));
    }
    if union.variants.is_empty() {
        return Err(type_::Error::Union(Error::NoVariants));
    }

    let match_expr = expr::lower_discriminants(&union.args, &struct_accum.done_fields)?;

    let start_byte: TokenStream = match &start_offset {
        GeneratedLen::Fixed(offset) => {
            if offset.bit != 0 {
                return Err(type_::Error::InvalidAlignment(*offset));
            }
            let byte = offset.byte;
            quote! { #byte }
        }
        GeneratedLen::Dynamic(tokens) => {
            quote! { (#tokens).byte }
        }
    };

    let parent_struct_name = &struct_accum.name;
    let field_name = &accum.field_name;
    let enum_name = format_ident!("{}_{}", parent_struct_name, field_name);

    let mut variant_structs = TokenStream::new();
    let mut enum_variants = TokenStream::new();
    let mut match_arms = TokenStream::new();
    let mut len_match_arms = TokenStream::new();

    for variant in &union.variants {
        let ast::UnionBody::NamedInline(inline_struct) = &variant.body else {
            todo!("@error union variants");
        };

        let variant_ident = format_ident!("{}", inline_struct.name);
        let struct_name = format_ident!("{}_{}_{}", parent_struct_name, field_name, inline_struct.name);

        let variant_attrs = crate::attr::ParsedAttrs::parse(&inline_struct.attributes)?;
        let variant_inherited = variant_attrs.merge_inherited(inherited);

        let (variant_struct, variant_len) = generate_variant_struct(&struct_name, &inline_struct.items, done, variant_inherited)?;
        variant_structs.extend(variant_struct);

        enum_variants.extend(quote! {
            #variant_ident(#struct_name<'a>),
        });

        let matchers = generate_matchers(&variant.matchers)?;
        let variant_len_byte = match variant_len {
            GeneratedLen::Fixed(len) => {
                let byte = len.byte;
                quote! { #byte }
            }
            GeneratedLen::Dynamic(tokens) => tokens,
        };

        match_arms.extend(quote! {
            #matchers => #enum_name::#variant_ident(#struct_name { data: &self.data[#start_byte..] }),
        });

        len_match_arms.extend(quote! {
            #matchers => ::binparse::Len { byte: #variant_len_byte, bit: 0 },
        });
    }

    struct_accum.other_entities.extend(quote! {
        #variant_structs

        #[allow(non_camel_case_types)]
        pub enum #enum_name<'a> {
            #enum_variants
        }
    });

    let field_getter_body = quote! {
        match #match_expr {
            #match_arms
        }
    };

    let len = GeneratedLen::Dynamic(quote! {
        match #match_expr {
            #len_match_arms
        }
    });

    Ok(GeneratedTypeInfo {
        len,
        field_getter_body,
        return_ty: quote! { #enum_name<'_> },
        field_type: DoneFieldType::Other,
    })
}

fn generate_variant_struct(
    struct_name: &syn::Ident,
    items: &[ast::StructItem<'_>],
    done: &HashMap<&str, GeneratedStruct>,
    inherited: Inherited,
) -> Result<(TokenStream, GeneratedLen), type_::Error> {
    let mut variant_accum = StructAccum::new(&struct_name.to_string(), inherited);

    for item in items {
        let ast::StructItem::Field(ast_field) = item else {
            todo!("conditional fields in union variants");
        };

        field::generate(ast_field, done, &mut variant_accum)
            .map_err(|e| type_::Error::Field(Box::new(e)))?;
    }

    let functions = variant_accum.functions;

    let variant_struct = quote! {
        #[allow(non_camel_case_types)]
        pub struct #struct_name<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }

        impl<'a> #struct_name<'a> {
            #functions
        }
    };

    Ok((variant_struct, variant_accum.offset))
}

fn generate_matchers(matchers: &[ast::UnionMatcher<'_>]) -> Result<TokenStream, Error> {
    let patterns = matchers.iter().map(generate_matcher).collect::<Vec<_>>();

    if patterns.len() != matchers.len()
        && !(matchers.len() == 1 && matches!(matchers[1], ast::UnionMatcher::Wildcard))
    {
        return Err(Error::MatcherCountMismatch {
            expected: matchers.len(),
            got: patterns.len(),
        });
    }

    Ok(quote! { #(#patterns)|* })
}

fn generate_matcher(matcher: &ast::UnionMatcher<'_>) -> TokenStream {
    match matcher {
        ast::UnionMatcher::Literal(ast::Literal::Int(int_lit)) => {
            let value = proc_macro2::Literal::usize_unsuffixed(int_lit.value);
            quote! { #value }
        }
        ast::UnionMatcher::Literal(other) => {
            todo!("non-integer literal matcher: {:?}", other)
        }
        ast::UnionMatcher::Wildcard => quote! { _ },
        ast::UnionMatcher::Tuple(elements) => {
            let element_patterns: Vec<_> = elements.iter().map(generate_matcher).collect();
            quote! { (#(#element_patterns),*) }
        }
    }
}
