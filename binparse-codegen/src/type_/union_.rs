use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    GeneratedLen,
    field::FieldCtx,
    struct_::{DoneField, GeneratedStruct},
};

use super::GeneratedType;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("union must have exactly one discriminant")]
    MultipleDiscriminants,
    #[error("union has no variants")]
    NoVariants,
}

pub(crate) struct UnionCtx<'a, 'b> {
    pub(crate) union: &'a ast::Union<'a>,
    pub(crate) field_name: &'b syn::Ident,
    pub(crate) parent_struct_name: &'b syn::Ident,
    pub(crate) start_offset: GeneratedLen,
    #[expect(dead_code)]
    pub(crate) done_fields: &'a [DoneField<'a>],
    pub(crate) done: &'b HashMap<&'a str, GeneratedStruct>,
}

impl UnionCtx<'_, '_> {
    pub(crate) fn generate(self) -> Result<GeneratedType, super::Error> {
        if self.union.args.len() != 1 {
            todo!("multi-discriminant unions");
        }

        let GeneratedLen::Fixed(start_offset) = self.start_offset else {
            todo!("dynamic start offset for unions");
        };

        if start_offset.bit != 0 {
            return Err(super::Error::InvalidAlignment(start_offset));
        }

        let discriminant = format_ident!("{}", self.union.args[0]);
        let enum_name = format_ident!("{}_{}", self.parent_struct_name, self.field_name);
        let start_byte = start_offset.byte;

        let mut variant_structs = TokenStream::new();
        let mut enum_variants = TokenStream::new();
        let mut match_arms = TokenStream::new();
        let mut len_match_arms = TokenStream::new();

        for variant in &self.union.variants {
            let ast::UnionBody::NamedInline(variant_name, items) = &variant.body else {
                todo!("@error union variants");
            };

            let variant_ident = format_ident!("{}", variant_name);
            let struct_name = format_ident!("{}_{}_{}", self.parent_struct_name, self.field_name, variant_name);

            let (variant_struct, variant_len) = self.generate_variant_struct(
                &struct_name,
                items,
            )?;
            variant_structs.extend(variant_struct);

            enum_variants.extend(quote! {
                #variant_ident(#struct_name<'a>),
            });

            let matchers = self.generate_matchers(&variant.matchers);
            let variant_len_byte = match &variant_len {
                GeneratedLen::Fixed(len) => {
                    let byte = len.byte;
                    quote! { #byte }
                }
                GeneratedLen::Dynamic(tokens) => tokens.clone(),
            };

            match_arms.extend(quote! {
                #matchers => #enum_name::#variant_ident(#struct_name { data: &self.data[#start_byte..] }),
            });

            len_match_arms.extend(quote! {
                #matchers => ::binparse::Len { byte: #variant_len_byte, bit: 0 },
            });
        }

        let helper_entities = quote! {
            #variant_structs

            pub enum #enum_name<'a> {
                #enum_variants
            }
        };

        let field_getter_body = quote! {
            match self.#discriminant() {
                #match_arms
            }
        };

        let len = GeneratedLen::Dynamic(quote! {
            match self.#discriminant() {
                #len_match_arms
            }
        });

        Ok(GeneratedType {
            len,
            definitions: quote! {},
            helper_fns: quote! {},
            helper_entities,
            field_getter_body,
            return_ty: quote! { #enum_name<'_> },
        })
    }

    fn generate_variant_struct(
        &self,
        struct_name: &syn::Ident,
        items: &[ast::StructItem<'_>],
    ) -> Result<(TokenStream, GeneratedLen), super::Error> {
        let mut functions = TokenStream::new();
        let mut offset = GeneratedLen::Fixed(Len { byte: 0, bit: 0 });
        let mut done_fields: Vec<DoneField> = vec![];

        for item in items {
            let ast::StructItem::Field(field) = item else {
                todo!("conditional fields in union variants");
            };

            let field_ctx = FieldCtx::new(field, offset.clone(), &done_fields, self.done, struct_name);
            let generated = field_ctx.generate().map_err(|e| super::Error::Field(Box::new(e)))?;

            functions.extend(generated.field_getter);
            functions.extend(generated.offset_getter);

            offset = offset + generated.len.clone();

            done_fields.push(DoneField {
                origin: field,
                len: generated.len,
                offset_getter_fn_name: generated.offset_getter_fn_name,
            });
        }

        let variant_struct = quote! {
            #[allow(non_camel_case_types)]
            pub struct #struct_name<'a> {
                data: &'a [u8],
            }

            impl<'a> #struct_name<'a> {
                #functions
            }
        };

        Ok((variant_struct, offset))
    }

    fn generate_matchers(&self, matchers: &[ast::Expr<'_>]) -> TokenStream {
        let patterns: Vec<TokenStream> = matchers
            .iter()
            .map(|m| self.generate_single_matcher(m))
            .collect();

        if patterns.len() == 1 {
            patterns.into_iter().next().unwrap()
        } else {
            quote! { #(#patterns)|* }
        }
    }

    fn generate_single_matcher(&self, expr: &ast::Expr<'_>) -> TokenStream {
        match expr {
            ast::Expr::Literal(ast::Literal::Int(int_lit)) => {
                let value = proc_macro2::Literal::usize_unsuffixed(int_lit.value);
                quote! { #value }
            }
            ast::Expr::Path(path) if path.len() == 1 && path[0] == "_" => {
                quote! { _ }
            }
            ast::Expr::Binary(binary) if matches!(binary.op, ast::BinaryOp::Numeric(ast::NumericBinaryOp::BitOr)) => {
                let lhs = self.generate_single_matcher(&binary.lhs);
                let rhs = self.generate_single_matcher(&binary.rhs);
                quote! { #lhs | #rhs }
            }
            other => todo!("complex union matchers: {:?}", other),
        }
    }
}
