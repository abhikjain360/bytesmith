use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    GeneratedLen,
    attr::{self, BitOrder, Endian, Inherited, ParsedAttrs},
    expr::{self, ExprType},
    field::FieldAccum,
    struct_::{DoneFieldType, GeneratedStruct, StructAccum},
    type_::{self, GeneratedTypeInfo},
};

enum SizeMode {
    Counted {
        count: TokenStream,
        static_count: Option<usize>,
    },
    Until(u8),
    Greedy,
}

struct ElemGen {
    len: GeneratedLen,
    return_ty: TokenStream,
    iterator_fields: TokenStream,
    iterator_init: TokenStream,
    next_fn_body: TokenStream,
    check_count: Option<TokenStream>,
}

pub(crate) fn generate(
    array_type: &ast::ArrayType<'_>,
    attrs: &ParsedAttrs<'_>,
    done: &HashMap<&str, GeneratedStruct>,
    struct_accum: &mut StructAccum,
    accum: &mut FieldAccum,
    start_offset: GeneratedLen,
    inherited: Inherited,
) -> Result<GeneratedTypeInfo, type_::Error> {
    let Inherited { endian, bit_order } = inherited;
    let field_name = accum.field_name.clone();

    let mode = match (&array_type.size, attrs.until, attrs.greedy) {
        (Some(size), None, false) => {
            let lowered = expr::lower(size, ExprType::Numeric, &struct_accum.done_fields)?;
            SizeMode::Counted {
                count: lowered.tokens,
                static_count: lowered.const_value,
            }
        }
        (Some(_), Some(_), _) => return Err(attr::Error::ArrayAttrOnSizedArray("until").into()),
        (Some(_), None, true) => return Err(attr::Error::ArrayAttrOnSizedArray("greedy").into()),
        (None, Some(_), true) => return Err(attr::Error::UntilWithGreedy.into()),
        (None, Some(sentinel), false) => SizeMode::Until(sentinel),
        (None, None, true) => SizeMode::Greedy,
        (None, None, false) => return Err(type_::Error::UnsizedArray),
    };
    let offset = match &start_offset {
        GeneratedLen::Fixed(len) => {
            if len.bit != 0 {
                return Err(type_::Error::InvalidAlignment(*len));
            }
            let byte = len.byte;
            quote! { #byte }
        }
        GeneratedLen::Dynamic(tokens) => {
            quote! {{
                let len = #tokens;
                if len.bit > 0 { return Err(::binparse::ParseError::UnalignedLength(len)) };
                len.byte
            }}
        }
    };
    let offset_bytes = match &start_offset {
        GeneratedLen::Fixed(len) => {
            let byte = len.byte;
            quote! { #byte }
        }
        GeneratedLen::Dynamic(tokens) => quote! { ({ #tokens }).byte_ceil() },
    };

    let iterator_name = format_ident!("{}_{}_Iterator", struct_accum.name, field_name);

    let bounded_next = |next_body: TokenStream| {
        quote! {
            if self.idx == self.count { return None; }
            self.idx += 1;
            #next_body
        }
    };

    let elem = match &array_type.elem_ty {
        ast::ArrayElemType::Primitive(prim) => {
            let (prim_len, prim_ty) = crate::match_primitive(prim);
            let byte_len = prim_len.byte;
            let count = match &mode {
                SizeMode::Counted { count, .. } => count.clone(),
                SizeMode::Until(sentinel) => {
                    if !matches!(prim, ast::Primitive::U8) {
                        todo!("@until on non-u8 element arrays");
                    }
                    until_count(&offset_bytes, *sentinel)
                }
                SizeMode::Greedy => greedy_count(&offset_bytes, byte_len),
            };
            let len = match &mode {
                SizeMode::Counted {
                    count,
                    static_count,
                } => counted_len(GeneratedLen::Fixed(prim_len), count, *static_count),
                SizeMode::Until(sentinel) => until_len(&offset_bytes, *sentinel),
                SizeMode::Greedy => greedy_len(&offset_bytes, byte_len),
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
            ElemGen {
                len,
                return_ty: prim_ty,
                iterator_fields: quote! {
                    idx: usize,
                    count: usize,
                    data: &'a [u8]
                },
                iterator_init: quote! {
                    idx: 0,
                    count: #count,
                    data: &self.data[#offset..],
                },
                next_fn_body: bounded_next(next_body),
                check_count: Some(count),
            }
        }

        ast::ArrayElemType::StructRef(struct_name) => {
            let generated_struct = done
                .get(*struct_name)
                .ok_or_else(|| type_::Error::UnknownType(struct_name.to_string()))?;
            let struct_ident = format_ident!("{}", struct_name);
            let return_ty = quote! { #struct_ident<'a> };
            let next_body = quote! {
                match #struct_ident::parse(self.data) {
                    Ok((value, rem)) => {
                        self.data = rem;
                        Some(Ok(value))
                    },
                    Err(error) => Some(Err(error)),
                }
            };
            match &mode {
                SizeMode::Counted {
                    count,
                    static_count,
                } => ElemGen {
                    len: counted_len(generated_struct.len.clone(), count, *static_count),
                    return_ty,
                    iterator_fields: quote! {
                        idx: usize,
                        count: usize,
                        data: &'a [u8]
                    },
                    iterator_init: quote! {
                        idx: 0,
                        count: #count,
                        data: &self.data[#offset..],
                    },
                    next_fn_body: bounded_next(next_body),
                    check_count: Some(count.clone()),
                },
                SizeMode::Until(_) => todo!("@until on struct ref arrays"),
                SizeMode::Greedy => match &generated_struct.len {
                    GeneratedLen::Fixed(elem_len) if *elem_len == Len::ZERO => {
                        return Err(type_::Error::GreedyZeroSizedElem);
                    }
                    GeneratedLen::Fixed(elem_len) if elem_len.bit == 0 => {
                        let count = greedy_count(&offset_bytes, elem_len.byte);
                        ElemGen {
                            len: greedy_len(&offset_bytes, elem_len.byte),
                            return_ty,
                            iterator_fields: quote! {
                                idx: usize,
                                count: usize,
                                data: &'a [u8]
                            },
                            iterator_init: quote! {
                                idx: 0,
                                count: #count,
                                data: &self.data[#offset..],
                            },
                            next_fn_body: bounded_next(next_body),
                            check_count: Some(count),
                        }
                    }
                    GeneratedLen::Fixed(_) => return Err(type_::Error::UnalignedType),
                    GeneratedLen::Dynamic(_) => {
                        let Some(max_expr) = &attrs.max_iter else {
                            return Err(attr::Error::GreedyRequiresMaxIter.into());
                        };
                        let max =
                            expr::lower(max_expr, ExprType::Numeric, &struct_accum.done_fields)?
                                .tokens;
                        let field_path = format!("{}.{}", struct_accum.name, field_name);
                        ElemGen {
                            len: GeneratedLen::Dynamic(quote! {{
                                let start = #offset_bytes;
                                ::binparse::Len { byte: self.data.len().saturating_sub(start), bit: 0 }
                            }}),
                            return_ty,
                            iterator_fields: quote! {
                                idx: usize,
                                max: usize,
                                data: &'a [u8]
                            },
                            iterator_init: quote! {
                                idx: 0,
                                max: #max,
                                data: &self.data[#offset..],
                            },
                            next_fn_body: quote! {
                                if self.data.is_empty() { return None; }
                                if self.idx == self.max {
                                    self.data = &[];
                                    return Some(Err(::binparse::ParseError::MaxIterationsExceeded {
                                        field: #field_path,
                                        max: self.max,
                                    }));
                                }
                                self.idx += 1;
                                match #struct_ident::parse(self.data) {
                                    Ok((value, rem)) => {
                                        self.data = rem;
                                        Some(Ok(value))
                                    },
                                    Err(error) => {
                                        self.data = &[];
                                        Some(Err(error))
                                    },
                                }
                            },
                            check_count: None,
                        }
                    }
                },
            }
        }

        ast::ArrayElemType::BitField(width) => {
            let SizeMode::Counted {
                count,
                static_count,
            } = &mode
            else {
                todo!("@until and @greedy on bitfield arrays");
            };
            let width = *width as usize;
            let len = Len {
                byte: 0,
                bit: width,
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
            ElemGen {
                len: counted_len(GeneratedLen::Fixed(len), count, *static_count),
                return_ty: quote! { u8 },
                iterator_fields: quote! {
                    idx: usize,
                    count: usize,
                    data: &'a [u8],
                    bit_offset: usize
                },
                iterator_init: quote! {
                    idx: 0,
                    count: #count,
                    data: &self.data[#offset..],
                    bit_offset: 0,
                },
                next_fn_body: bounded_next(next_body),
                check_count: Some(count.clone()),
            }
        }
    };

    let ElemGen {
        len,
        return_ty: elem_return_ty,
        iterator_fields,
        iterator_init,
        next_fn_body,
        check_count,
    } = elem;

    struct_accum.other_entities.extend(quote! {
        #[allow(non_camel_case_types)]
        pub struct #iterator_name<'a> {
            #iterator_fields
        }

        impl<'a> ::std::iter::Iterator for #iterator_name<'a> {
            type Item = ::binparse::ParseResult<#elem_return_ty>;

            fn next(&mut self) -> std::option::Option<Self::Item> {
                #next_fn_body
            }
        }
    });

    if let Some(max_expr) = &attrs.max_iter
        && let Some(check_count) = &check_count
    {
        if struct_accum.condition.is_some() {
            todo!("@max_iter inside conditionals");
        }
        let max = expr::lower(max_expr, ExprType::Numeric, &struct_accum.done_fields)?.tokens;
        let validate_fn_name = format_ident!("{}_max_iter_validate", field_name);
        let field_path = format!("{}.{}", struct_accum.name, field_name);
        accum.helper_fns.extend(quote! {
            fn #validate_fn_name(&self) -> Result<(), ::binparse::ParseError> {
                let count = #check_count;
                let max = #max;
                if count > max {
                    return Err(::binparse::ParseError::MaxIterationsExceeded {
                        field: #field_path,
                        max,
                    });
                }
                Ok(())
            }
        });
        accum.parse_checks.extend(quote! {
            me.#validate_fn_name()?;
        });
    }

    let field_getter_body = quote! {
        Ok(#iterator_name {
            #iterator_init
        })
    };

    Ok(GeneratedTypeInfo {
        len,
        field_getter_body,
        return_ty: quote! { ::binparse::ParseResult<#iterator_name<'_>> },
        field_type: DoneFieldType::Other,
    })
}

fn counted_len(
    elem_len: GeneratedLen,
    count: &TokenStream,
    static_count: Option<usize>,
) -> GeneratedLen {
    match static_count {
        Some(static_count) => elem_len * static_count,
        None => match elem_len {
            GeneratedLen::Fixed(Len { byte, bit }) => GeneratedLen::Dynamic(quote! {
                ::binparse::Len { byte: #byte, bit: #bit } * (#count)
            }),
            GeneratedLen::Dynamic(a) => GeneratedLen::Dynamic(quote! { (#a) * (#count) }),
        },
    }
}

fn until_count(offset_bytes: &TokenStream, sentinel: u8) -> TokenStream {
    quote! {
        self.data[#offset_bytes..]
            .iter()
            .position(|&b| b == #sentinel)
            .unwrap_or(0)
    }
}

fn until_len(offset_bytes: &TokenStream, sentinel: u8) -> GeneratedLen {
    GeneratedLen::Dynamic(quote! {{
        let start = #offset_bytes;
        let byte = match self
            .data
            .get(start..)
            .and_then(|rest| rest.iter().position(|&b| b == #sentinel))
        {
            Some(pos) => pos.saturating_add(1),
            None => self.data.len().saturating_add(1).saturating_sub(start),
        };
        ::binparse::Len { byte, bit: 0 }
    }})
}

fn greedy_count(offset_bytes: &TokenStream, elem_byte_len: usize) -> TokenStream {
    if elem_byte_len == 1 {
        quote! { self.data.len().saturating_sub(#offset_bytes) }
    } else {
        quote! { (self.data.len().saturating_sub(#offset_bytes)) / #elem_byte_len }
    }
}

fn greedy_len(offset_bytes: &TokenStream, elem_byte_len: usize) -> GeneratedLen {
    if elem_byte_len == 1 {
        GeneratedLen::Dynamic(quote! {{
            let start = #offset_bytes;
            ::binparse::Len { byte: self.data.len().saturating_sub(start), bit: 0 }
        }})
    } else {
        GeneratedLen::Dynamic(quote! {{
            let start = #offset_bytes;
            let rem = self.data.len().saturating_sub(start);
            let extra = rem % #elem_byte_len;
            let byte = if extra > 0 {
                (rem - extra).saturating_add(#elem_byte_len)
            } else {
                rem
            };
            ::binparse::Len { byte, bit: 0 }
        }})
    }
}
