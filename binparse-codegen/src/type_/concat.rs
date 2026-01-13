use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::struct_::{DoneField, GeneratedStruct};

use super::{GeneratedType, TypeCtx};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("concat item {0} must have known length")]
    UnknownItemLen(usize),
}

pub(crate) struct ConcatCtx<'a, 'b> {
    pub(crate) items: &'a [ast::ConcatItem<'a>],
    pub(crate) field_name: &'b syn::Ident,
    pub(crate) start_offset: Option<Len>,
    pub(crate) prev_field: Option<&'a DoneField<'a>>,
    pub(crate) done: &'b std::collections::HashMap<&'a str, GeneratedStruct>,
}

impl ConcatCtx<'_, '_> {
    pub(crate) fn generate(self) -> Result<GeneratedType, super::Error> {
        match self.start_offset {
            Some(start_offset) => {
                let mut total_len = Len::default();
                let mut field_types = Vec::new();
                let mut field_exprs = TokenStream::new();
                let mut definitions = TokenStream::new();
                let mut helper_fns = TokenStream::new();
                let mut helper_entities = TokenStream::new();

                let mut current_offset = start_offset;

                for (i, item) in self.items.iter().enumerate() {
                    let item_name = format_ident!("{}_{}", self.field_name, i);

                    let GeneratedType {
                        len: item_len,
                        definitions: type_definitions,
                        helper_fns: type_helper_fns,
                        helper_entities: type_helper_entities,
                        field_getter_body,
                        return_ty,
                    } = TypeCtx { done: self.done }.generate(
                        &item.ty,
                        &item_name,
                        Some(current_offset),
                        self.prev_field,
                    )?;

                    let item_len = item_len.ok_or(Error::UnknownItemLen(i))?;

                    definitions.extend(type_definitions);
                    helper_fns.extend(type_helper_fns);
                    helper_fns.extend(quote! {
                        #[allow(clippy::identity_op)]
                        pub fn #item_name(&self) -> #return_ty {
                            #field_getter_body
                        }
                    });
                    helper_entities.extend(type_helper_entities);
                    field_types.push(return_ty);
                    field_exprs.extend(quote! { self.#item_name(), });

                    total_len = total_len + item_len;
                    current_offset = current_offset + item_len;
                }

                let field_getter_body = quote! {
                    ( #field_exprs )
                };

                let return_ty = quote! { ( #(#field_types),* ) };

                Ok(GeneratedType {
                    len: Some(total_len),
                    definitions,
                    helper_fns,
                    helper_entities,
                    field_getter_body,
                    return_ty,
                })
            }

            None => todo!(),
        }
    }
}
