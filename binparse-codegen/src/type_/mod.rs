use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;

use crate::{
    GeneratedLen,
    struct_::{DoneField, GeneratedStruct},
};

pub(crate) mod array;
pub(crate) mod bitfield;
pub(crate) mod concat;
pub(crate) mod primitive;
pub(crate) mod struct_ref;
pub(crate) mod union_;

pub(crate) struct GeneratedType {
    pub(crate) len: GeneratedLen,
    pub(crate) definitions: TokenStream,
    pub(crate) helper_fns: TokenStream,
    pub(crate) helper_entities: TokenStream,
    pub(crate) field_getter_body: TokenStream,
    pub(crate) return_ty: TokenStream,
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
    #[error(transparent)]
    Concat(#[from] concat::Error),
    #[error(transparent)]
    Array(#[from] array::Error),
    #[error(transparent)]
    Union(#[from] union_::Error),
    #[error("field error: {0}")]
    Field(Box<crate::field::Error>),
}

pub(crate) struct TypeCtx<'a, 'b> {
    pub(crate) done: &'a HashMap<&'a str, GeneratedStruct>,
    pub(crate) parent_struct_name: &'b syn::Ident,
}

impl<'a, 'b> TypeCtx<'a, 'b> {
    pub(crate) fn generate(
        &self,
        ty: &'a ast::Type<'a>,
        field_name: &syn::Ident,
        start_offset: GeneratedLen,
        done_fields: &'a [DoneField<'a>],
    ) -> Result<GeneratedType, Error> {
        match ty {
            ast::Type::Primitive(p) => primitive::PrimitiveCtx {
                primitive: p,
                start_offset,
            }
            .generate(),

            ast::Type::BitField(width) => bitfield::BitFieldCtx {
                width: *width as usize,
                start_offset,
            }
            .generate(),

            ast::Type::Concat(items) => concat::ConcatCtx {
                items,
                field_name,
                start_offset,
                done_fields,
                done: self.done,
                parent_struct_name: self.parent_struct_name,
            }
            .generate(),

            ast::Type::StructRef(struct_name) => struct_ref::StructRefCtx {
                struct_name,
                start_offset,
                done: self.done,
            }
            .generate(),

            ast::Type::Array(array_type) => array::ArrayCtx {
                array_type,
                field_name,
                done_fields,
                start_offset,
                done: self.done,
            }
            .generate(),

            ast::Type::Union(u) => union_::UnionCtx {
                union: u,
                field_name,
                parent_struct_name: self.parent_struct_name,
                start_offset,
                done_fields,
                done: self.done,
            }
            .generate(),
        }
    }
}
