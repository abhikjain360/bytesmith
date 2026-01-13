use binparse::Len;
use binparse_dsl as ast;
use quote::{format_ident, quote};

use crate::struct_::{DoneField, GeneratedStruct};

use super::{Error, GeneratedType, primitive::match_primitive};

pub(crate) struct ArrayCtx<'a, 'b> {
    pub(crate) array_type: &'a ast::ArrayType<'a>,
    pub(crate) field_name: &'b syn::Ident,
    pub(crate) start_offset: Option<Len>,
    pub(crate) prev_field: Option<&'a DoneField<'a>>,
    pub(crate) done: &'b std::collections::HashMap<&'a str, GeneratedStruct>,
}

impl ArrayCtx<'_, '_> {
    pub(crate) fn generate(self) -> Result<GeneratedType, Error> {
        match self.start_offset {
            Some(start_offset) => {
                if start_offset.bit != 0 {
                    return Err(Error::InvalidAlignment(start_offset));
                }

                let count = self.generate_array_size(&self.array_type.size);

                let offset = match self.prev_field {
                    Some(DoneField {
                        offset_getter_fn_name,
                        ..
                    }) => quote! {{
                        let len = self.#offset_getter_fn_name();
                        if len.bit > 0 { return Err(::binparse::ParseError::UnalignedLength(len)) };
                        len.byte
                    }},
                    None => quote! { 0 },
                };

                let iterator_name = format_ident!("{}_Iterator", self.field_name);

                let (elem_len, elem_return_ty, iterator_fields, iterator_init, next_body) =
                    match &self.array_type.elem_ty {
                        ast::ArrayElemType::Primitive(prim) => {
                            let (len, prim_ty) = match_primitive(prim);
                            let byte_len = len.byte;
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
                                let value = #prim_ty::from_ne_bytes(self.data[..#byte_len].try_into().unwrap());
                                self.data = &self.data[#byte_len..];
                                Some(Ok(value))
                            };
                            (len, prim_ty, iterator_fields, iterator_init, next_body)
                        }

                        ast::ArrayElemType::StructRef(struct_name) => {
                            let generated_struct = self
                                .done
                                .get(*struct_name)
                                .ok_or_else(|| Error::UnknownType(struct_name.to_string()))?;
                            let len = generated_struct.len.ok_or(Error::UnsizedType)?;
                            let struct_ident = format_ident!("{}", struct_name);
                            let return_ty = quote! { #struct_ident<'_> };
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
                                let ret = #struct_ident::parse(self.data);
                                if let Ok((rem, _)) = &ret {
                                    self.data = rem;
                                }
                                Some(ret)
                            };
                            (len, return_ty, iterator_fields, iterator_init, next_body)
                        }

                        ast::ArrayElemType::BitField(width) => {
                            let width = *width as usize;
                            let len = Len { byte: 0, bit: width };
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
                            let next_body = quote! {
                                let byte_idx = self.bit_offset / 8;
                                let bit_idx = self.bit_offset % 8;
                                let value = if bit_idx + #width <= 8 {
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
                                };
                                self.bit_offset += #width;
                                Some(Ok(value))
                            };
                            (len, return_ty, iterator_fields, iterator_init, next_body)
                        }
                    };

                let helper_entities = quote! {
                    #[allow(non_camel_case_types)]
                    struct #iterator_name<'a> {
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
                };

                let field_getter_body = quote! {
                    #iterator_name {
                        #iterator_init
                    }
                };

                Ok(GeneratedType {
                    len: Some(elem_len * count),
                    definitions: quote! {},
                    helper_fns: quote! {},
                    helper_entities,
                    field_getter_body,
                    return_ty: quote! { #iterator_name },
                })
            }

            None => todo!(),
        }
    }

    fn generate_array_size(&self, size: &ast::ArraySize) -> usize {
        match size {
            ast::ArraySize::Unsized => todo!("try from attributes"),
            ast::ArraySize::Path(_) => todo!(),

            ast::ArraySize::Int(ast::IntLiteral { value, .. }) => *value,

            ast::ArraySize::Binary(_) => todo!(),
        }
    }
}
