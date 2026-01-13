use binparse::Len;
use quote::quote;

use super::{Error, GeneratedType};

pub(super) struct BitFieldCtx<'a> {
    pub(super) width: usize,
    pub(super) field_name: &'a syn::Ident,
    pub(super) start_offset: Option<Len>,
}

impl BitFieldCtx<'_> {
    pub(super) fn generate(self) -> Result<GeneratedType, Error> {
        let len = Len {
            byte: 0,
            bit: self.width,
        };
        let return_ty = quote! { u8 };

        match self.start_offset {
            Some(offset) => {
                let start_byte = offset.byte;
                let start_bit = offset.bit;
                let field_name = self.field_name;

                let field_getter = if start_bit + self.width <= 8 {
                    let mask = (1u8 << self.width) - 1;
                    quote! {
                        #[allow(clippy::identity_op)]
                        pub fn #field_name(&self) -> u8 {
                            (self.data[#start_byte] >> #start_bit) & #mask
                        }
                    }
                } else {
                    let bits_in_first_byte = 8 - start_bit;
                    let bits_in_second_byte = self.width - bits_in_first_byte;
                    let first_mask = (1u8 << bits_in_first_byte) - 1;
                    let second_mask = (1u8 << bits_in_second_byte) - 1;
                    let second_byte = start_byte + 1;

                    quote! {
                        #[allow(clippy::identity_op)]
                        pub fn #field_name(&self) -> u8 {
                            let first_part = (self.data[#start_byte] >> #start_bit) & #first_mask;
                            let second_part = self.data[#second_byte] & #second_mask;
                            first_part | (second_part << #bits_in_first_byte)
                        }
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
