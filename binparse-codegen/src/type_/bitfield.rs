use binparse::Len;
use quote::quote;

use crate::{
    GeneratedLen,
    struct_::DoneFieldType,
    type_::{Error, GeneratedTypeInfo},
};

pub(crate) fn generate(
    width: usize,
    start_offset: GeneratedLen,
) -> Result<GeneratedTypeInfo, Error> {
    let len = Len {
        byte: 0,
        bit: width,
    };
    let return_ty = quote! { u8 };
    let mask = (1u8 << width) - 1;

    let field_getter_body = match start_offset {
        GeneratedLen::Fixed(offset) => {
            let start_byte = offset.byte;
            let start_bit = offset.bit;

            if start_bit + width <= 8 {
                quote! {
                    (self.data[#start_byte] >> #start_bit) & #mask
                }
            } else {
                let bits_in_first_byte = 8 - start_bit;
                let bits_in_second_byte = width - bits_in_first_byte;
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
            }
        }

        GeneratedLen::Dynamic(offset_expr) => {
            quote! {
                {
                    let offset = #offset_expr;
                    let start_byte = offset.byte;
                    let start_bit = offset.bit;
                    if start_bit + #width <= 8 {
                        (self.data[start_byte] >> start_bit) & #mask
                    } else {
                        let bits_in_first = 8 - start_bit;
                        let first_mask = (1u8 << bits_in_first) - 1;
                        let second_mask = (1u8 << (#width - bits_in_first)) - 1;
                        let first_part = (self.data[start_byte] >> start_bit) & first_mask;
                        let second_part = self.data[start_byte + 1] & second_mask;
                        first_part | (second_part << bits_in_first)
                    }
                }
            }
        }
    };

    Ok(GeneratedTypeInfo {
        len: GeneratedLen::Fixed(len),
        field_getter_body,
        return_ty,
        field_type: DoneFieldType::BitField,
    })
}
