use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use quote::{format_ident, quote};

use crate::{
    GeneratedLen,
    attr::{BitOrder, Endian, Inherited},
    expr::{self, ExprType},
    field::FieldAccum,
    struct_::{DoneFieldType, GeneratedStruct, StructAccum},
    type_::{self, GeneratedTypeInfo},
};

pub(crate) fn generate(
    array_type: &ast::ArrayType<'_>,
    done: &HashMap<&str, GeneratedStruct>,
    struct_accum: &mut StructAccum,
    accum: &mut FieldAccum,
    start_offset: GeneratedLen,
    inherited: Inherited,
) -> Result<GeneratedTypeInfo, type_::Error> {
    let Inherited { endian, bit_order } = inherited;
    let field_name = &accum.field_name;

    let (count, static_count) = match &array_type.size {
        Some(size) => {
            let lowered = expr::lower(size, ExprType::Numeric, &struct_accum.done_fields)?;
            (lowered.tokens, lowered.const_value)
        }
        None => todo!("try from attributes"),
    };
    let done_fields = &struct_accum.done_fields;

    let offset = match &start_offset {
        GeneratedLen::Fixed(len) => {
            if len.bit != 0 {
                return Err(type_::Error::InvalidAlignment(*len));
            }
            let byte = len.byte;
            quote! { #byte }
        }
        GeneratedLen::Dynamic(_) => {
            let offset_getter_fn_name = &done_fields
                .last()
                .expect("dynamic offset requires previous field")
                .offset_getter_fn_name;
            quote! {{
                let len = self.#offset_getter_fn_name();
                if len.bit > 0 { return Err(::binparse::ParseError::UnalignedLength(len)) };
                len.byte
            }}
        }
    };

    let iterator_name = format_ident!("{}_{}_Iterator", struct_accum.name, field_name);

    let (elem_len, elem_return_ty, iterator_fields, iterator_init, next_body) =
        match &array_type.elem_ty {
            ast::ArrayElemType::Primitive(prim) => {
                let (prim_len, prim_ty) = crate::match_primitive(prim);
                let byte_len = prim_len.byte;
                let iterator_fields = quote! {
                    idx: usize,
                    count: usize,
                    data: &'a [u8]
                };
                let iterator_init = quote! {
                    idx: 0,
                    count: #count,
                    data: &self.data[#offset..],
                };
                let next_body = if let Some(read) = crate::single_byte_read(prim) {
                    quote! {
                        let value = self.data[0] #read;
                        self.data = &self.data[1..];
                        Some(Ok(value))
                    }
                } else {
                    let from_bytes = match endian {
                        Endian::Big => quote! { from_be_bytes },
                        Endian::Little => quote! { from_le_bytes },
                    };
                    quote! {
                        let value = #prim_ty::#from_bytes(self.data[..#byte_len].try_into().unwrap());
                        self.data = &self.data[#byte_len..];
                        Some(Ok(value))
                    }
                };
                (
                    GeneratedLen::Fixed(prim_len),
                    prim_ty,
                    iterator_fields,
                    iterator_init,
                    next_body,
                )
            }

            ast::ArrayElemType::StructRef(struct_name) => {
                let generated_struct = done
                    .get(*struct_name)
                    .ok_or_else(|| type_::Error::UnknownType(struct_name.to_string()))?;
                let struct_ident = format_ident!("{}", struct_name);
                let return_ty = quote! { #struct_ident<'a> };
                let iterator_fields = quote! {
                    idx: usize,
                    count: usize,
                    data: &'a [u8]
                };
                let iterator_init = quote! {
                    idx: 0,
                    count: #count,
                    data: &self.data[#offset..],
                };
                let next_body = quote! {
                    match #struct_ident::parse(self.data) {
                        Ok((value, rem)) => {
                            self.data = rem;
                            Some(Ok(value))
                        },
                        Err(error) => Some(Err(error)),
                    }
                };
                (
                    generated_struct.len.clone(),
                    return_ty,
                    iterator_fields,
                    iterator_init,
                    next_body,
                )
            }

            ast::ArrayElemType::BitField(width) => {
                let width = *width as usize;
                let len = Len {
                    byte: 0,
                    bit: width,
                };
                let return_ty = quote! { u8 };
                let iterator_fields = quote! {
                    idx: usize,
                    count: usize,
                    data: &'a [u8],
                    bit_offset: usize
                };
                let iterator_init = quote! {
                    idx: 0,
                    count: #count,
                    data: &self.data[#offset..],
                    bit_offset: 0,
                };
                let extract = match bit_order {
                    BitOrder::Msb => quote! {
                        if bit_idx + #width <= 8 {
                            let mask = (1u8 << #width) - 1;
                            (self.data[byte_idx] >> (8 - bit_idx - #width)) & mask
                        } else {
                            let bits_in_first = 8 - bit_idx;
                            let bits_in_second = #width - bits_in_first;
                            let first_mask = (1u8 << bits_in_first) - 1;
                            let first_part = self.data[byte_idx] & first_mask;
                            let second_part = self.data[byte_idx + 1] >> (8 - bits_in_second);
                            (first_part << bits_in_second) | second_part
                        }
                    },
                    BitOrder::Lsb => quote! {
                        if bit_idx + #width <= 8 {
                            let mask = (1u8 << #width) - 1;
                            (self.data[byte_idx] >> bit_idx) & mask
                        } else {
                            let bits_in_first = 8 - bit_idx;
                            let bits_in_second = #width - bits_in_first;
                            let first_mask = (1u8 << bits_in_first) - 1;
                            let second_mask = (1u8 << bits_in_second) - 1;
                            let first_part = (self.data[byte_idx] >> bit_idx) & first_mask;
                            let second_part = self.data[byte_idx + 1] & second_mask;
                            first_part | (second_part << bits_in_first)
                        }
                    },
                };
                let next_body = quote! {
                    let byte_idx = self.bit_offset / 8;
                    let bit_idx = self.bit_offset % 8;
                    let value = #extract;
                    self.bit_offset += #width;
                    Some(Ok(value))
                };
                (
                    GeneratedLen::Fixed(len),
                    return_ty,
                    iterator_fields,
                    iterator_init,
                    next_body,
                )
            }
        };

    struct_accum.other_entities.extend(quote! {
        #[allow(non_camel_case_types)]
        pub struct #iterator_name<'a> {
            #iterator_fields
        }

        impl<'a> ::std::iter::Iterator for #iterator_name<'a> {
            type Item = ::binparse::ParseResult<#elem_return_ty>;

            fn next(&mut self) -> std::option::Option<Self::Item> {
                if self.idx == self.count { return None; }
                self.idx += 1;
                #next_body
            }
        }
    });

    let field_getter_body = quote! {
        Ok(#iterator_name {
            #iterator_init
        })
    };

    let len = match static_count {
        Some(count) => elem_len * count,
        None => match elem_len {
            GeneratedLen::Fixed(Len { byte, bit }) => GeneratedLen::Dynamic(quote! {
                ::binparse::Len { byte: #byte, bit: #bit } * (#count)
            }),
            GeneratedLen::Dynamic(a) => GeneratedLen::Dynamic(quote! { (#a) * (#count) }),
        },
    };

    Ok(GeneratedTypeInfo {
        len,
        field_getter_body,
        return_ty: quote! { ::binparse::ParseResult<#iterator_name<'_>> },
        field_type: DoneFieldType::Other,
    })
}
