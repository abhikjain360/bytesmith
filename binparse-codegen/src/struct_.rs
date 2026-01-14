use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{GeneratedLen, field::FieldCtx};

pub(crate) struct StructCtx<'a> {
    pub(crate) origin: &'a ast::Struct<'a>,
    pub(crate) offset: GeneratedLen,
    done_fields: Vec<DoneField<'a>>,
    pub(crate) done: &'a HashMap<&'a str, GeneratedStruct>,
}

#[expect(dead_code)]
pub(crate) struct DoneField<'a> {
    pub(crate) origin: &'a ast::Field<'a>,
    pub(crate) len: GeneratedLen,
    pub(crate) offset_getter_fn_name: syn::Ident,
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
}

impl<'a> StructCtx<'a> {
    pub(crate) fn new(
        origin: &'a ast::Struct<'a>,
        done: &'a HashMap<&'a str, GeneratedStruct>,
    ) -> Self {
        Self {
            origin,
            offset: GeneratedLen::Fixed(Len { byte: 0, bit: 0 }),
            done_fields: vec![],
            done,
        }
    }

    pub(crate) fn generate(mut self) -> Result<GeneratedStruct, Error> {
        let mut field_definitions = TokenStream::new();
        let mut functions = TokenStream::new();
        let mut other_entities = TokenStream::new();

        let mut last_offset_getter_fn_name = None;

        let name = format_ident!("{}", self.origin.name);

        for item in &self.origin.items {
            if let ast::StructItem::Field(field) = item {
                let current_offset = self.offset.clone();
                let field_ctx = FieldCtx::new(field, current_offset, &self.done_fields, self.done, &name);
                let generated = field_ctx.generate().map_err(|error| Error::Field {
                    name: field.name.to_string(),
                    error,
                })?;

                field_definitions.extend(generated.definitions);
                functions.extend(generated.helper_fns);
                functions.extend(generated.field_getter);
                functions.extend(generated.offset_getter);
                other_entities.extend(generated.helper_entities);

                self.offset = self.offset + generated.len.clone();

                last_offset_getter_fn_name = Some(generated.offset_getter_fn_name.clone());

                self.done_fields.push(DoneField {
                    origin: field,
                    len: generated.len,
                    offset_getter_fn_name: generated.offset_getter_fn_name,
                });
            }
        }

        let parse_fn = if let Some(fn_name) = last_offset_getter_fn_name {
            quote! {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
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
                data: &'a [u8],
                #field_definitions
            }

            impl<'a> #name<'a> {
                #parse_fn

                #functions
            }
        };

        Ok(GeneratedStruct {
            len: self.offset,
            tokens,
        })
    }
}
