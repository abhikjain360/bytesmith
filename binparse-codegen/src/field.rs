use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::struct_::{DoneField, GeneratedStruct};
use crate::type_;

pub(crate) struct FieldCtx<'a> {
    pub(crate) field: &'a ast::Field<'a>,
    pub(crate) start_offset: Option<Len>,
    pub(crate) done_fields: &'a [DoneField<'a>],
    pub(crate) done: &'a HashMap<&'a str, GeneratedStruct>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("type generation error: {0}")]
    Type(#[from] type_::Error),
    #[error("cannot determine field offset: no start offset and no previous fields")]
    UnknownOffset,
}

pub(crate) struct GeneratedField {
    pub(crate) len: Option<Len>,
    pub(crate) offset_getter_fn_name: syn::Ident,
    pub(crate) definitions: TokenStream,
    pub(crate) helper_fns: TokenStream,
    pub(crate) helper_entities: TokenStream,
    pub(crate) field_getter: TokenStream,
    pub(crate) offset_getter: TokenStream,
}

impl<'a> FieldCtx<'a> {
    pub(crate) fn new(
        field: &'a ast::Field<'a>,
        start_offset: Option<Len>,
        done_fields: &'a [DoneField<'a>],
        done: &'a HashMap<&'a str, GeneratedStruct>,
    ) -> Self {
        Self {
            field,
            start_offset,
            done_fields,
            done,
        }
    }

    pub(crate) fn generate(self) -> Result<GeneratedField, Error> {
        let field_name = format_ident!("{}", self.field.name);
        let offset_getter_fn_name = format_ident!("{}_end_offset", field_name);

        let (len, definitions, helper_fns, helper_entities, field_getter) = match &self.field.value
        {
            ast::FieldValue::Type(ty) => {
                let generated = type_::TypeCtx { done: self.done }.generate(
                    ty,
                    &field_name,
                    self.start_offset,
                    self.done_fields,
                )?;
                let return_ty = generated.return_ty;
                let field_getter_body = generated.field_getter_body;
                let field_getter = quote! {
                    #[allow(clippy::identity_op)]
                    pub fn #field_name(&self) -> #return_ty {
                        #field_getter_body
                    }
                };
                (
                    generated.len,
                    generated.definitions,
                    generated.helper_fns,
                    generated.helper_entities,
                    field_getter,
                )
            }

            ast::FieldValue::Constraint(_) => todo!(),
        };

        let offset_getter = match (&self.start_offset, self.done_fields.last()) {
            (Some(offset), _) => match &len {
                Some(len) => {
                    let total_len = *offset + *len;
                    let total_byte = total_len.byte;
                    let total_bit = total_len.bit;

                    quote! {
                        pub fn #offset_getter_fn_name(&self) -> binparse::Len {
                            binparse::Len {
                                byte: #total_byte,
                                bit: #total_bit,
                            }
                        }
                    }
                }

                None => todo!(),
            },

            (None, Some(prev_field)) => {
                let prev_offset_getter = &prev_field.offset_getter_fn_name;
                match &len {
                    Some(len) => {
                        let len_byte = len.byte;
                        let len_bit = len.bit;

                        quote! {
                            pub fn #offset_getter_fn_name(&self) -> binparse::Len {
                                let prev = self.#prev_offset_getter();
                                binparse::Len {
                                    byte: prev.byte + #len_byte,
                                    bit: prev.bit + #len_bit,
                                }
                            }
                        }
                    }

                    None => todo!(),
                }
            }

            (None, None) => return Err(Error::UnknownOffset),
        };

        Ok(GeneratedField {
            len,
            definitions,
            helper_fns,
            helper_entities,
            offset_getter_fn_name,
            offset_getter,
            field_getter,
        })
    }
}
