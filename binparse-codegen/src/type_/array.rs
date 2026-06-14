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
    type_::{self, GeneratedTree, GeneratedTypeInfo, LenBound},
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

pub(crate) fn generate<'a>(
    array_type: &ast::ArrayType<'a>,
    attrs: &ParsedAttrs<'a>,
    done: &HashMap<&'a str, GeneratedStruct>,
    struct_accum: &mut StructAccum<'a>,
    accum: &mut FieldAccum,
    start_offset: GeneratedLen,
    inherited: Inherited,
) -> Result<GeneratedTypeInfo, type_::Error> {
    let Inherited { endian, bit_order } = inherited;
    let field_name = accum.field_name.clone();

    let mut fill_to_bound = false;
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
        (None, None, false) if struct_accum.struct_len.is_some() => {
            fill_to_bound = true;
            SizeMode::Greedy
        }
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

    let bound = type_::len_bound(&offset_bytes, attrs, struct_accum)?;
    if bound.is_some() && !matches!(mode, SizeMode::Until(_) | SizeMode::Greedy) {
        return Err(attr::Error::LenOnSizedArray.into());
    }
    let struct_bound_end = match (&bound, &struct_accum.struct_len) {
        (Some(_), _) => None,
        (None, Some(len_expr)) if matches!(mode, SizeMode::Until(_) | SizeMode::Greedy) => {
            let lowered = expr::lower(len_expr, ExprType::Numeric, &struct_accum.done_fields)?;
            let len_tokens = lowered.tokens;
            Some(quote! { (#len_tokens).min(self.data.len()) })
        }
        (None, _) => None,
    };
    let data = match (&bound, &struct_bound_end) {
        (Some(LenBound { end, .. }), _) => quote! { self.data[..(#end)] },
        (None, Some(end)) => quote! { self.data[..(#end)] },
        (None, None) => quote! { self.data },
    };
    if fill_to_bound {
        struct_accum.fill_to_bound_field = Some(field_name.to_string());
    }

    let iterator_name = format_ident!("{}_{}_Iterator", struct_accum.name, field_name);

    let bounded_next = |next_body: TokenStream| {
        quote! {
            if self.idx == self.count { return None; }
            self.idx += 1;
            #next_body
        }
    };

    let elem = match &array_type.elem_ty.kind {
        ast::ArrayElemTypeKind::Primitive(prim) => {
            let (prim_len, prim_ty) = crate::match_primitive(prim);
            let byte_len = prim_len.byte;
            let count = match &mode {
                SizeMode::Counted { count, .. } => count.clone(),
                SizeMode::Until(sentinel) => {
                    if !matches!(prim, ast::Primitive::U8) {
                        return Err(type_::Error::UntilOnNonU8Array);
                    }
                    until_count(&data, &offset_bytes, *sentinel)
                }
                SizeMode::Greedy => greedy_count(&data, &offset_bytes, byte_len),
            };
            let len = match &mode {
                SizeMode::Counted {
                    count,
                    static_count,
                } => counted_len(GeneratedLen::Fixed(prim_len), count, *static_count),
                SizeMode::Until(sentinel) => until_len(&data, &offset_bytes, *sentinel),
                SizeMode::Greedy => greedy_len(&data, &offset_bytes, byte_len),
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
                    data: &#data[#offset..],
                },
                next_fn_body: bounded_next(next_body),
                check_count: Some(count),
            }
        }

        ast::ArrayElemTypeKind::StructRef(struct_name) => {
            let generated_struct = done
                .get(struct_name.text)
                .ok_or_else(|| type_::Error::UnknownType(struct_name.text.to_string()))?;
            let struct_ident = format_ident!("{}", struct_name.text);
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
                        data: &#data[#offset..],
                    },
                    next_fn_body: bounded_next(next_body),
                    check_count: Some(count.clone()),
                },
                SizeMode::Until(_) => return Err(type_::Error::UntilOnStructRefArray),
                SizeMode::Greedy => match &generated_struct.len {
                    GeneratedLen::Fixed(elem_len) if *elem_len == Len::ZERO => {
                        return Err(type_::Error::GreedyZeroSizedElem);
                    }
                    GeneratedLen::Fixed(elem_len) if elem_len.bit == 0 => {
                        let count = greedy_count(&data, &offset_bytes, elem_len.byte);
                        ElemGen {
                            len: greedy_len(&data, &offset_bytes, elem_len.byte),
                            return_ty,
                            iterator_fields: quote! {
                                idx: usize,
                                count: usize,
                                data: &'a [u8]
                            },
                            iterator_init: quote! {
                                idx: 0,
                                count: #count,
                                data: &#data[#offset..],
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
                                ::binparse::Len { byte: #data.len().saturating_sub(start), bit: 0 }
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
                                data: &#data[#offset..],
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

        ast::ArrayElemTypeKind::BitField(width) => {
            let SizeMode::Counted {
                count,
                static_count,
            } = &mode
            else {
                return Err(type_::Error::UntilOrGreedyOnBitfieldArray);
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
                    data: &#data[#offset..],
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
            return Err(type_::Error::MaxIterInConditional);
        }
        let max = expr::lower(max_expr, ExprType::Numeric, &struct_accum.done_fields)?.tokens;
        let validate_fn_name = format_ident!("{}_max_iter_validate", field_name);
        let field_path = format!("{}.{}", struct_accum.name, field_name);
        accum.helper_fns.extend(quote! {
            fn #validate_fn_name(&mut self) -> Result<(), ::binparse::ParseError> {
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
            self.#validate_fn_name()?;
        });
    }

    let field_getter_body = quote! {
        Ok(#iterator_name {
            #iterator_init
        })
    };

    let field_len = match &bound {
        Some(LenBound { end, field_len }) => {
            let consumed = match &len {
                GeneratedLen::Fixed(len) => {
                    let byte = len.byte;
                    quote! { #byte }
                }
                GeneratedLen::Dynamic(tokens) => quote! { ({ #tokens }).byte_ceil() },
            };
            let rest_fn_name = format_ident!("{}_rest", field_name);
            let (vis, dead_code) = crate::field::getter_visibility(attrs);
            accum.helper_fns.extend(quote! {
                #dead_code
                #vis fn #rest_fn_name(&mut self) -> ::binparse::ParseResult<&'a [u8]> {
                    let end = #end;
                    let consumed = (#offset_bytes).saturating_add(#consumed);
                    if consumed > end {
                        return Err(::binparse::ParseError::NotEnoughData {
                            expected: consumed,
                            got: end,
                        });
                    }
                    Ok(&self.data[consumed..end])
                }
            });
            if matches!(mode, SizeMode::Until(_)) {
                accum.pre_length_checks.extend(quote! {
                    self.#rest_fn_name()?;
                });
            }
            field_len.clone()
        }
        None => len,
    };

    let tree = GeneratedTree::Node(if bound.is_some() {
        let rest_fn_name = format_ident!("{}_rest", field_name);
        let inner = tree_node(array_type, done, &field_name, &accum.tree_getter)?;
        quote! {
            {
                let mut node = #inner;
                if let Ok(rest) = self.#rest_fn_name()
                    && !rest.is_empty()
                {
                    let consumed = node
                        .children
                        .last()
                        .map(|child| child.bit_range.end)
                        .unwrap_or(bit_range.start)
                        .min(bit_range.end);
                    node.children.push(::binparse::FieldNode::new(
                        "rest",
                        "[u8]",
                        consumed..bit_range.end,
                        ::binparse::Value::Bytes(rest),
                    ));
                }
                node
            }
        }
    } else {
        tree_node(array_type, done, &field_name, &accum.tree_getter)?
    });

    Ok(GeneratedTypeInfo {
        len: field_len,
        field_getter_body,
        return_ty: quote! { ::binparse::ParseResult<#iterator_name<'a>> },
        field_type: DoneFieldType::Other,
        tree,
    })
}

fn tree_node(
    array_type: &ast::ArrayType<'_>,
    done: &HashMap<&str, GeneratedStruct>,
    field_name: &syn::Ident,
    getter: &syn::Ident,
) -> Result<TokenStream, type_::Error> {
    let name_str = field_name.to_string();
    let elem_label = type_::elem_label(&array_type.elem_ty);
    let type_label = format!("[{elem_label}]");

    let error_node = quote! {
        elem_nodes.push(
            ::binparse::FieldNode::new(
                    i.to_string(),
                    #elem_label,
                    start..start,
                    ::binparse::Value::Opaque,
                )
                .with_status(::binparse::Status::Error(error)),
        );
    };

    let elem_loop = match &array_type.elem_ty.kind {
        ast::ArrayElemTypeKind::Primitive(prim) => {
            let elem_bits = crate::match_primitive(prim).0.bits();
            let value_ctor = if crate::is_signed(prim) {
                quote! { ::binparse::Value::Int(i128::from(value)) }
            } else {
                quote! { ::binparse::Value::UInt(u128::from(value)) }
            };
            elem_value_loop(
                getter,
                quote! { #elem_bits },
                &elem_label,
                value_ctor,
                &error_node,
            )
        }
        ast::ArrayElemTypeKind::BitField(width) => {
            let elem_bits = *width as usize;
            let value_ctor = quote! { ::binparse::Value::UInt(u128::from(value)) };
            elem_value_loop(
                getter,
                quote! { #elem_bits },
                &elem_label,
                value_ctor,
                &error_node,
            )
        }
        ast::ArrayElemTypeKind::StructRef(struct_name) => {
            let generated_struct = done
                .get(struct_name.text)
                .ok_or_else(|| type_::Error::UnknownType(struct_name.text.to_string()))?;
            let elem_len = match &generated_struct.last_offset_getter {
                Some(getter) => quote! { value.#getter().bits() },
                None => quote! { 0usize },
            };
            quote! {
                if let Ok(iter) = self.#getter() {
                    let mut start = bit_range.start;
                    for (i, elem) in iter.enumerate() {
                        match elem {
                            Ok(mut value) => {
                                let end = start.saturating_add(#elem_len);
                                elem_nodes.push(value.field_tree().renamed(i.to_string()).shifted(start));
                                start = end;
                            }
                            Err(error) => {
                                #error_node
                            }
                        }
                    }
                }
            }
        }
    };

    Ok(quote! {
        {
            let mut elem_nodes = ::std::vec::Vec::new();
            #elem_loop
            ::binparse::FieldNode::new(#name_str, #type_label, bit_range.clone(), ::binparse::Value::Array)
                .with_children(elem_nodes)
        }
    })
}

fn elem_value_loop(
    getter: &syn::Ident,
    elem_bits: TokenStream,
    elem_label: &str,
    value_ctor: TokenStream,
    error_node: &TokenStream,
) -> TokenStream {
    quote! {
        if let Ok(iter) = self.#getter() {
            let mut start = bit_range.start;
            for (i, elem) in iter.enumerate() {
                let end = start.saturating_add(#elem_bits);
                match elem {
                    Ok(value) => elem_nodes.push(::binparse::FieldNode::new(
                        i.to_string(),
                        #elem_label,
                        start..end,
                        #value_ctor,
                    )),
                    Err(error) => {
                        #error_node
                    }
                }
                start = end;
            }
        }
    }
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

fn until_count(data: &TokenStream, offset_bytes: &TokenStream, sentinel: u8) -> TokenStream {
    quote! {
        #data[#offset_bytes..]
            .iter()
            .position(|&b| b == #sentinel)
            .unwrap_or(0)
    }
}

fn until_len(data: &TokenStream, offset_bytes: &TokenStream, sentinel: u8) -> GeneratedLen {
    GeneratedLen::Dynamic(quote! {{
        let start = #offset_bytes;
        let byte = match #data
            .get(start..)
            .and_then(|rest| rest.iter().position(|&b| b == #sentinel))
        {
            Some(pos) => pos.saturating_add(1),
            None => #data.len().saturating_add(1).saturating_sub(start),
        };
        ::binparse::Len { byte, bit: 0 }
    }})
}

fn greedy_count(
    data: &TokenStream,
    offset_bytes: &TokenStream,
    elem_byte_len: usize,
) -> TokenStream {
    if elem_byte_len == 1 {
        quote! { #data.len().saturating_sub(#offset_bytes) }
    } else {
        quote! { (#data.len().saturating_sub(#offset_bytes)) / #elem_byte_len }
    }
}

fn greedy_len(
    data: &TokenStream,
    offset_bytes: &TokenStream,
    elem_byte_len: usize,
) -> GeneratedLen {
    if elem_byte_len == 1 {
        GeneratedLen::Dynamic(quote! {{
            let start = #offset_bytes;
            ::binparse::Len { byte: #data.len().saturating_sub(start), bit: 0 }
        }})
    } else {
        GeneratedLen::Dynamic(quote! {{
            let start = #offset_bytes;
            let rem = #data.len().saturating_sub(start);
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
