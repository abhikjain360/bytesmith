use std::collections::HashMap;

use bytesmith::Len;
use quote::{format_ident, quote};

use crate::{
    GeneratedLen,
    attr::ParsedAttrs,
    expr::{self, ExprType},
    field::{FieldAccum, getter_visibility},
    struct_::{DoneFieldType, GeneratedStruct, StructAccum},
    type_::{Error, GeneratedTree, GeneratedTypeInfo},
};

pub(crate) fn generate<'a>(
    struct_name: &str,
    done: &HashMap<&'a str, GeneratedStruct>,
    struct_accum: &mut StructAccum<'a>,
    accum: &mut FieldAccum,
    start_offset: GeneratedLen,
    attrs: &ParsedAttrs<'a>,
) -> Result<GeneratedTypeInfo, Error> {
    let generated_struct = done
        .get(struct_name)
        .ok_or_else(|| Error::UnknownType(struct_name.to_string()))?;

    let struct_ident = format_ident!("{}", struct_name);
    let return_ty = quote! { ::bytesmith::ParseResult<#struct_ident<'a>> };
    let field_ident = accum.tree_getter.clone();
    let name_str = accum.field_name.to_string();

    if let Some(len_expr) = &attrs.len {
        let lowered = expr::lower(len_expr, ExprType::Numeric, &struct_accum.done_fields)?;
        if let (Some(bound), GeneratedLen::Fixed(inner_len)) =
            (lowered.const_value, &generated_struct.len)
            && bound < inner_len.byte_ceil()
        {
            return Err(Error::LenBoundTooSmall {
                bound,
                needed: inner_len.byte_ceil(),
            });
        }

        let start = match &start_offset {
            GeneratedLen::Fixed(offset) => {
                if offset.bit != 0 {
                    return Err(Error::InvalidAlignment(*offset));
                }
                let byte = offset.byte;
                quote! { #byte }
            }
            GeneratedLen::Dynamic(tokens) => quote! {{
                let len = #tokens;
                if len.bit > 0 { return Err(::bytesmith::ParseError::UnalignedLength(len)) };
                len.byte
            }},
        };

        let len_tokens = lowered.tokens;
        let field_getter_body = quote! {
            let start = #start;
            let end = start.saturating_add(#len_tokens);
            #struct_ident::parse(&self.data[start..end]).map(|(value, _)| value)
        };

        let rest_fn_name = format_ident!("{}_rest", accum.field_name);
        let (vis, dead_code) = getter_visibility(attrs);
        accum.helper_fns.extend(quote! {
            #dead_code
            #vis fn #rest_fn_name(&mut self) -> ::bytesmith::ParseResult<&'a [u8]> {
                let start = #start;
                let end = start.saturating_add(#len_tokens);
                #struct_ident::parse(&self.data[start..end]).map(|(_, rest)| rest)
            }
        });

        let len = match lowered.const_value {
            Some(byte) => GeneratedLen::Fixed(Len { byte, bit: 0 }),
            None => GeneratedLen::Dynamic(quote! {
                ::bytesmith::Len { byte: #len_tokens, bit: 0 }
            }),
        };

        let tree = GeneratedTree::Node(quote! {
            match self.#field_ident() {
                Ok(mut value) => {
                    let inner = value.field_tree().renamed(#name_str).shifted(bit_range.start);
                    let consumed = inner.bit_range.end.min(bit_range.end);
                    let mut node = inner.with_bit_range(bit_range.clone());
                    if let Ok(rest) = self.#rest_fn_name()
                        && !rest.is_empty()
                    {
                        node.children.push(::bytesmith::FieldNode::new(
                            "rest",
                            "[u8]",
                            consumed..bit_range.end,
                            ::bytesmith::Value::Bytes(rest),
                        ));
                    }
                    node
                }
                Err(error) => ::bytesmith::FieldNode::new(
                        #name_str,
                        #struct_name,
                        bit_range.clone(),
                        ::bytesmith::Value::Opaque,
                    )
                    .with_status(::bytesmith::Status::Error(error)),
            }
        });

        return Ok(GeneratedTypeInfo {
            len,
            field_getter_body,
            return_ty,
            field_type: DoneFieldType::Other,
            tree,
        });
    }

    let start_byte = match &start_offset {
        GeneratedLen::Fixed(offset) => {
            if offset.bit != 0 {
                return Err(Error::InvalidAlignment(*offset));
            }
            let byte = offset.byte;
            quote! { #byte }
        }
        GeneratedLen::Dynamic(tokens) => quote! {{
            let len = #tokens;
            if len.bit > 0 { return Err(::bytesmith::ParseError::UnalignedLength(len)) };
            len.byte
        }},
    };

    let len = match &generated_struct.len {
        GeneratedLen::Fixed(len) => GeneratedLen::Fixed(*len),
        GeneratedLen::Dynamic(_) => {
            let last_offset_getter = generated_struct
                .last_offset_getter
                .clone()
                .expect("dynamic-length struct ref has an offset getter");
            GeneratedLen::Dynamic(quote! {{
                let start = (#start_byte).min(self.data.len());
                match #struct_ident::parse(&self.data[start..]) {
                    Ok((mut value, _)) => value.#last_offset_getter(),
                    Err(_) => bytesmith::Len::ZERO,
                }
            }})
        }
    };

    let field_getter_body = quote! {
        #struct_ident::parse(&self.data[#start_byte..]).map(|(value, _)| value)
    };

    let tree = GeneratedTree::Node(quote! {
        match self.#field_ident() {
            Ok(mut value) => value.field_tree().renamed(#name_str).shifted(bit_range.start),
            Err(error) => ::bytesmith::FieldNode::new(
                    #name_str,
                    #struct_name,
                    bit_range.clone(),
                    ::bytesmith::Value::Opaque,
                )
                .with_status(::bytesmith::Status::Error(error)),
        }
    });

    Ok(GeneratedTypeInfo {
        len,
        field_getter_body,
        return_ty,
        field_type: DoneFieldType::Other,
        tree,
    })
}
