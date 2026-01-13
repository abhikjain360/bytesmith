use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;

use crate::struct_::{DoneField, GeneratedStruct};

pub(crate) mod array;
pub(crate) mod bitfield;
pub(crate) mod concat;
pub(crate) mod primitive;
pub(crate) mod struct_ref;

pub(crate) struct GeneratedType {
    pub(crate) len: Option<Len>,
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
}

pub(crate) struct TypeCtx<'a> {
    pub(crate) done: &'a HashMap<&'a str, GeneratedStruct>,
}

impl<'a> TypeCtx<'a> {
    pub(crate) fn generate(
        &self,
        ty: &ast::Type,
        field_name: &syn::Ident,
        start_offset: Option<Len>,
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

            _ => todo!(),
        }
    }
}
