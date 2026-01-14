use binparse::Len;
use quote::quote;

use crate::{
    GeneratedLen,
    type_::{Error, GeneratedType},
};

pub(crate) struct BitFieldCtx {
    pub(crate) width: usize,
    pub(crate) start_offset: GeneratedLen,
}

impl BitFieldCtx {
    pub(crate) fn generate(self) -> Result<GeneratedType, Error> {
        let len = Len {
            byte: 0,
            bit: self.width,
        };
        let return_ty = quote! { u8 };

        match self.start_offset {
            GeneratedLen::Fixed(offset) => {
                let start_byte = offset.byte;
                let start_bit = offset.bit;

                let field_getter_body = if start_bit + self.width <= 8 {
                    let mask = (1u8 << self.width) - 1;
                    quote! {
                        (self.data[#start_byte] >> #start_bit) & #mask
                    }
                } else {
                    let bits_in_first_byte = 8 - start_bit;
                    let bits_in_second_byte = self.width - bits_in_first_byte;
                    let first_mask = (1u8 << bits_in_first_byte) - 1;
                    let second_mask = (1u8 << bits_in_second_byte) - 1;
                    let second_byte = start_byte + 1;

                    quote! {
                        {
                            let first_part = (self.data[#start_byte] >> #start_bit) & #first_mask;
                            let second_part = self.data[#second_byte] & #second_mask;
                            first_part | (second_part << #bits_in_first_byte)
                        }
                    }
                };

                Ok(GeneratedType {
                    len: GeneratedLen::Fixed(len),
                    definitions: quote! {},
                    helper_fns: quote! {},
                    helper_entities: quote! {},
                    field_getter_body,
                    return_ty,
                })
            }

            GeneratedLen::Dynamic(_) => todo!(),
        }
    }
}
