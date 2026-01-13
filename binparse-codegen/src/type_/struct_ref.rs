use std::collections::HashMap;

use binparse::Len;
use quote::{format_ident, quote};

use crate::struct_::GeneratedStruct;

use super::{Error, GeneratedType};

pub(super) struct StructRefCtx<'a, 'b> {
    pub(super) struct_name: &'a str,
    pub(super) field_name: &'b syn::Ident,
    pub(super) start_offset: Option<Len>,
    pub(super) done: &'b HashMap<&'a str, GeneratedStruct>,
}

impl StructRefCtx<'_, '_> {
    pub(super) fn generate(self) -> Result<GeneratedType, Error> {
        let generated_struct = self
            .done
            .get(self.struct_name)
            .ok_or_else(|| Error::UnknownType(self.struct_name.to_string()))?;

        let len = generated_struct.len;
        let struct_ident = format_ident!("{}", self.struct_name);
        let return_ty = quote! { #struct_ident<'_> };

        match self.start_offset {
            Some(offset) => {
                if offset.bit != 0 {
                    return Err(Error::InvalidAlignment(offset));
                }
                let start_byte = offset.byte;
                let field_name = self.field_name;

                let field_getter = quote! {
                    pub fn #field_name(&self) -> #struct_ident<'_> {
                        #struct_ident::parse(&self.data[#start_byte..]).unwrap().0
                    }
                };

                Ok(GeneratedType {
                    len,
                    definitions: quote! {},
                    field_getter,
                    return_ty,
                })
            }

            None => todo!(),
        }
    }
}
