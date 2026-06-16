use std::collections::HashMap;

use bytesmith::Len;
use bytesmith_dsl as ast;
use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    GeneratedLen,
    attr::{Inherited, ParsedAttrs},
    field::FieldAccum,
    struct_::{DoneFieldType, GeneratedStruct, StructAccum},
};

pub(crate) mod array;
pub(crate) mod bitfield;
pub(crate) mod concat;
pub(crate) mod primitive;
pub(crate) mod struct_ref;
pub(crate) mod union_;

pub(crate) struct GeneratedTypeInfo {
    pub(crate) len: GeneratedLen,
    pub(crate) field_getter_body: TokenStream,
    pub(crate) return_ty: TokenStream,
    pub(crate) field_type: DoneFieldType,
    pub(crate) tree: GeneratedTree,
}

pub(crate) enum GeneratedTree {
    UInt(String),
    Int(String),
    Node(TokenStream),
}

impl GeneratedTypeInfo {
    pub(crate) fn tree_node(&self, field_name: &syn::Ident, getter: &syn::Ident) -> TokenStream {
        let name = field_name.to_string();
        match &self.tree {
            GeneratedTree::UInt(type_name) => quote! {
                ::bytesmith::FieldNode::new(
                    #name,
                    #type_name,
                    bit_range.clone(),
                    ::bytesmith::Value::UInt(u128::from(self.#getter())),
                )
            },
            GeneratedTree::Int(type_name) => quote! {
                ::bytesmith::FieldNode::new(
                    #name,
                    #type_name,
                    bit_range.clone(),
                    ::bytesmith::Value::Int(i128::from(self.#getter())),
                )
            },
            GeneratedTree::Node(tokens) => tokens.clone(),
        }
    }
}

pub(crate) fn opaque_node(name: &str, type_name: &str) -> TokenStream {
    quote! {
        ::bytesmith::FieldNode::new(#name, #type_name, bit_range.clone(), ::bytesmith::Value::Opaque)
    }
}

pub(crate) struct LenBound {
    pub(crate) end: TokenStream,
    pub(crate) field_len: GeneratedLen,
}

pub(crate) fn len_bound(
    start: &TokenStream,
    attrs: &ParsedAttrs<'_>,
    struct_accum: &StructAccum<'_>,
) -> Result<Option<LenBound>, Error> {
    let Some(len_expr) = &attrs.len else {
        return Ok(None);
    };
    let lowered = crate::expr::lower(
        len_expr,
        crate::expr::ExprType::Numeric,
        &struct_accum.done_fields,
    )?;
    let len_tokens = lowered.tokens;
    let end = quote! {
        ({ #start }).saturating_add(#len_tokens).min(self.data.len())
    };
    let field_len = match lowered.const_value {
        Some(byte) => GeneratedLen::Fixed(Len { byte, bit: 0 }),
        None => GeneratedLen::Dynamic(quote! {
            ::bytesmith::Len { byte: #len_tokens, bit: 0 }
        }),
    };
    Ok(Some(LenBound { end, field_len }))
}

pub(crate) fn type_label(ty: &ast::Type<'_>) -> String {
    match &ty.kind {
        ast::TypeKind::Primitive(p) => crate::match_primitive(p).1.to_string(),
        ast::TypeKind::BitField(width) => format!("b<{width}>"),
        ast::TypeKind::Array(array) => format!("[{}]", elem_label(&array.elem_ty)),
        ast::TypeKind::StructRef(name) => name.text.to_string(),
        ast::TypeKind::Concat(_) => "concat".to_string(),
        ast::TypeKind::Union(_) => "union".to_string(),
    }
}

pub(crate) fn elem_label(elem: &ast::ArrayElemType<'_>) -> String {
    match &elem.kind {
        ast::ArrayElemTypeKind::Primitive(p) => crate::match_primitive(p).1.to_string(),
        ast::ArrayElemTypeKind::BitField(width) => format!("b<{width}>"),
        ast::ArrayElemTypeKind::StructRef(name) => name.text.to_string(),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("type needs alignment but is not aligned itself")]
    UnalignedType,
    #[error("type must have a known size")]
    UnsizedType,
    #[error("type needs alignment, but the start offset ({0:?}) is not aligned")]
    InvalidAlignment(Len),
    #[error("unknown type: {0}")]
    UnknownType(String),
    #[error("array without size requires @until, @greedy, or @hook")]
    UnsizedArray,
    #[error("@len({bound}) is smaller than the referenced struct's fixed length of {needed} bytes")]
    LenBoundTooSmall { bound: usize, needed: usize },
    #[error("@greedy element type has zero length")]
    GreedyZeroSizedElem,
    #[error("@until is not supported on arrays of non-u8 elements")]
    UntilOnNonU8Array,
    #[error("@until is not supported on arrays of struct refs")]
    UntilOnStructRefArray,
    #[error("@until and @greedy are not supported on bitfield arrays")]
    UntilOrGreedyOnBitfieldArray,
    #[error("@max_iter is not supported inside conditionals")]
    MaxIterInConditional,
    #[error(transparent)]
    Concat(#[from] concat::Error),
    #[error(transparent)]
    Expr(#[from] crate::expr::Error),
    #[error(transparent)]
    Union(#[from] union_::Error),
    #[error("field error: {0}")]
    Field(Box<crate::field::Error>),
    #[error(transparent)]
    Attr(#[from] crate::attr::Error),
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate<'a>(
    ast: &ast::Type<'a>,
    done: &HashMap<&'a str, GeneratedStruct>,
    struct_accum: &mut StructAccum<'a>,
    field_accum: &mut FieldAccum,
    start_offset: GeneratedLen,
    inherited: Inherited,
    attrs: &ParsedAttrs<'a>,
    errors: &[ast::ErrorVariant<'_>],
) -> Result<GeneratedTypeInfo, Error> {
    match &ast.kind {
        ast::TypeKind::Primitive(p) => primitive::generate(*p, start_offset, inherited.endian),
        ast::TypeKind::BitField(width) => {
            bitfield::generate(*width as usize, start_offset, inherited.bit_order)
        }
        ast::TypeKind::Concat(items) => concat::generate(
            items,
            done,
            struct_accum,
            field_accum,
            start_offset,
            inherited,
            errors,
        ),
        ast::TypeKind::StructRef(struct_name) => struct_ref::generate(
            struct_name.text,
            done,
            struct_accum,
            field_accum,
            start_offset,
            attrs,
        ),
        ast::TypeKind::Array(array_type) => array::generate(
            array_type,
            attrs,
            done,
            struct_accum,
            field_accum,
            start_offset,
            inherited,
        ),
        ast::TypeKind::Union(u) => union_::generate(
            u,
            done,
            struct_accum,
            field_accum,
            start_offset,
            inherited,
            attrs,
            errors,
        ),
    }
}
