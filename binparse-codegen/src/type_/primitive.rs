use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::quote;

use super::{Error, GeneratedType};

pub(super) struct PrimitiveCtx<'a> {
    pub(super) primitive: &'a ast::Primitive,
    pub(super) field_name: &'a syn::Ident,
    pub(super) start_offset: Option<Len>,
}

impl PrimitiveCtx<'_> {
    pub(super) fn generate(self) -> Result<GeneratedType, Error> {
        let (len, def) = match_primitive(self.primitive);
        let return_ty = def.clone();

        match self.start_offset {
            Some(offset) => {
                if offset.bit != 0 {
                    return Err(Error::InvalidAlignment(offset));
                }

                let end = offset + len;
                let start_byte = offset.byte;
                let end_byte = end.byte;
                let field_name = self.field_name;

                let field_getter = quote! {
                    pub fn #field_name(&self) -> #def {
                        #def::from_ne_bytes(self.data[#start_byte..#end_byte].try_into().unwrap())
                    }
                };

                Ok(GeneratedType {
                    len: Some(len),
                    definitions: quote! {},
                    field_getter,
                    return_ty,
                })
            }

            None => todo!(),
        }
    }
}

pub(super) fn match_primitive(primitive: &ast::Primitive) -> (Len, TokenStream) {
    match primitive {
        ast::Primitive::U8 => (Len { byte: 1, bit: 0 }, quote! { u8 }),
        ast::Primitive::U16 => (Len { byte: 2, bit: 0 }, quote! { u16 }),
        ast::Primitive::U32 => (Len { byte: 4, bit: 0 }, quote! { u32 }),
        ast::Primitive::U64 => (Len { byte: 8, bit: 0 }, quote! { u64 }),
        ast::Primitive::U128 => (Len { byte: 16, bit: 0 }, quote! { u128 }),
    }
}
