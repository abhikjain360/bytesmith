use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;

use crate::struct_::GeneratedStruct;

mod bitfield;
mod concat;
mod primitive;
mod struct_ref;

pub(crate) struct GeneratedType {
    pub(crate) len: Option<Len>,
    pub(crate) definitions: TokenStream,
    pub(crate) field_getter: TokenStream,
    pub(crate) return_ty: TokenStream,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("type needs alignment but is not aligned itself")]
    UnalignedType,
    #[error("type needs alignment, but the start offset ({0:?}) is not aligned")]
    InvalidAlignment(Len),
    #[error("unknown type: {0}")]
    UnknownType(String),
}

pub(crate) fn generate(
    ty: &ast::Type,
    field_name: &syn::Ident,
    start_offset: Option<Len>,
    done: &HashMap<&str, GeneratedStruct>,
) -> Result<GeneratedType, Error> {
    match ty {
        ast::Type::Primitive(p) => primitive::PrimitiveCtx {
            primitive: p,
            field_name,
            start_offset,
        }
        .generate(),

        ast::Type::BitField(width) => bitfield::BitFieldCtx {
            width: *width as usize,
            field_name,
            start_offset,
        }
        .generate(),

        ast::Type::Concat(items) => concat::ConcatCtx {
            items,
            field_name,
            start_offset,
            done,
        }
        .generate(),

        ast::Type::StructRef(struct_name) => struct_ref::StructRefCtx {
            struct_name,
            field_name,
            start_offset,
            done,
        }
        .generate(),

        _ => todo!(),
    }
}
