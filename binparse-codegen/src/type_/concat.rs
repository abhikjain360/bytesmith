use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::struct_::GeneratedStruct;

use super::{Error, GeneratedType};

pub(super) struct ConcatCtx<'a, 'b> {
    pub(super) items: &'a [ast::ConcatItem<'a>],
    pub(super) field_name: &'b syn::Ident,
    pub(super) start_offset: Option<Len>,
    pub(super) done: &'b HashMap<&'a str, GeneratedStruct>,
}

impl ConcatCtx<'_, '_> {
    pub(super) fn generate(self) -> Result<GeneratedType, Error> {
        match self.start_offset {
            Some(start_offset) => {
                let mut total_len = Len::default();
                let mut field_types = Vec::new();
                let mut field_exprs = TokenStream::new();
                let mut sub_getters = TokenStream::new();

                let mut current_offset = start_offset;

                for (i, item) in self.items.iter().enumerate() {
                    let item_name = format_ident!("{}_{}", self.field_name, i);

                    let GeneratedType {
                        len: item_len,
                        definitions,
                        field_getter,
                        return_ty,
                    } = super::generate(&item.ty, &item_name, Some(current_offset), self.done)?;

                    let item_len = item_len.expect("concat items should have known length");

                    sub_getters.extend(definitions);
                    sub_getters.extend(field_getter);
                    field_types.push(return_ty);
                    field_exprs.extend(quote! { self.#item_name(), });

                    total_len = total_len + item_len;
                    current_offset = current_offset + item_len;
                }

                let field_name = self.field_name;
                let field_getter = quote! {
                    #sub_getters

                    pub fn #field_name(&self) -> ( #(#field_types),* ) {
                        ( #field_exprs )
                    }
                };

                let return_ty = quote! { ( #(#field_types),* ) };

                Ok(GeneratedType {
                    len: Some(total_len),
                    definitions: quote! {},
                    field_getter,
                    return_ty,
                })
            }

            None => todo!(),
        }
    }
}
