use std::collections::HashMap;

use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    GeneratedLen,
    attr::{self, Hook, ParsedAttrs},
    struct_::{DoneField, DoneFieldType, GeneratedStruct, StructAccum},
    type_,
};

pub(crate) struct FieldAccum {
    pub(crate) field_name: syn::Ident,
    pub(crate) len: GeneratedLen,
    pub(crate) field_type: DoneFieldType,
    pub(crate) offset_getter_fn_name: syn::Ident,
    pub(crate) definitions: TokenStream,
    pub(crate) helper_fns: TokenStream,
    pub(crate) field_getter: TokenStream,
    pub(crate) offset_getter: TokenStream,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("type generation error: {0}")]
    Type(#[from] type_::Error),
    #[error("cannot determine field offset: no start offset and no previous fields")]
    UnknownOffset,
    #[error(transparent)]
    Attr(#[from] attr::Error),
}

impl FieldAccum {
    pub(crate) fn new(field_name: &str) -> Self {
        let field_name_ident = format_ident!("{}", field_name);
        let offset_getter_fn_name = format_ident!("{}_end_offset", field_name);
        Self {
            field_name: field_name_ident,
            len: GeneratedLen::Fixed(binparse::Len { byte: 0, bit: 0 }),
            field_type: DoneFieldType::Other,
            offset_getter_fn_name,
            definitions: TokenStream::new(),
            helper_fns: TokenStream::new(),
            field_getter: TokenStream::new(),
            offset_getter: TokenStream::new(),
        }
    }
}

pub(crate) fn generate<'a>(
    ast: &ast::Field<'a>,
    done: &HashMap<&'a str, GeneratedStruct>,
    struct_accum: &mut StructAccum,
) -> Result<(), Error> {
    let attrs = ParsedAttrs::parse(&ast.attributes)?;
    let field_endian = attrs.merge_endian(struct_accum.endian);
    let mut field_accum = FieldAccum::new(ast.name);

    match &ast.value {
        ast::FieldValue::Type(ty) => {
            if attrs.endian.is_some() {
                match ty {
                    ast::Type::Primitive(ast::Primitive::U8) => {
                        return Err(attr::Error::EndianOnU8.into())
                    }
                    ast::Type::BitField(_) => return Err(attr::Error::EndianOnBitfield.into()),
                    ast::Type::StructRef(_) => return Err(attr::Error::EndianOnStructRef.into()),
                    _ => {}
                }
            }

            match (&attrs.hook, is_vla(ty)) {
                (Some(hook), true) => {
                    generate_vla_hook(hook, struct_accum, &mut field_accum)?;
                }
                (Some(hook), false) => {
                    let start_offset = struct_accum.offset.clone();
                    generate_fixed_hook(
                        hook,
                        ty,
                        done,
                        struct_accum,
                        &mut field_accum,
                        start_offset,
                        field_endian,
                    )?;
                }
                (None, _) => {
                    let start_offset = struct_accum.offset.clone();
                    let info = type_::generate(
                        ty,
                        done,
                        struct_accum,
                        &mut field_accum,
                        start_offset,
                        field_endian,
                    )?;

                    field_accum.len = info.len;
                    field_accum.field_type = info.field_type;

                    let field_name = &field_accum.field_name;
                    let return_ty = info.return_ty;
                    let field_getter_body = info.field_getter_body;
                    field_accum.field_getter = quote! {
                        #[allow(clippy::identity_op)]
                        pub fn #field_name(&self) -> #return_ty {
                            #field_getter_body
                        }
                    };
                }
            }
        }

        ast::FieldValue::Constraint(_) => todo!("handle constraint-type fields"),
    };

    let offset_getter_fn_name = field_accum.offset_getter_fn_name;
    let len = field_accum.len;
    let field_type = field_accum.field_type;

    let start_offset = std::mem::replace(
        &mut struct_accum.offset,
        GeneratedLen::Fixed(binparse::Len { byte: 0, bit: 0 }),
    );
    let end_offset = start_offset + len.clone();

    let start_offset_getter_fn_name = format_ident!("{}_start_offset", ast.name);
    let bit_range_fn_name = format_ident!("{}_bit_range", ast.name);
    let start_offset_body = match &struct_accum.last_offset_getter_fn_name {
        Some(prev) => quote! { self.#prev() },
        None => quote! { binparse::Len::ZERO },
    };

    field_accum.offset_getter = match &end_offset {
        GeneratedLen::Fixed(total_len) => {
            let total_byte = total_len.byte;
            let total_bit = total_len.bit;
            quote! {
                pub fn #offset_getter_fn_name(&self) -> binparse::Len {
                    binparse::Len { byte: #total_byte, bit: #total_bit }
                }
            }
        }
        GeneratedLen::Dynamic(total_len) => {
            quote! {
                pub fn #offset_getter_fn_name(&self) -> binparse::Len {
                    #total_len
                }
            }
        }
    };
    field_accum.offset_getter.extend(quote! {
        pub fn #start_offset_getter_fn_name(&self) -> binparse::Len {
            #start_offset_body
        }

        pub fn #bit_range_fn_name(&self) -> ::core::ops::Range<usize> {
            self.#start_offset_getter_fn_name().bits()..self.#offset_getter_fn_name().bits()
        }
    });

    struct_accum.offset = end_offset;
    struct_accum.field_definitions.extend(field_accum.definitions);
    struct_accum.functions.extend(field_accum.helper_fns);
    struct_accum.functions.extend(field_accum.field_getter);
    struct_accum.functions.extend(field_accum.offset_getter);
    struct_accum.parse_checks.extend(quote! {
        {
            let len = me.#offset_getter_fn_name();
            let expected = len.byte_ceil();
            if data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: data.len(),
                });
            }
        }
    });
    struct_accum.last_offset_getter_fn_name = Some(offset_getter_fn_name.clone());
    struct_accum.done_fields.push(DoneField {
        name: ast.name.to_string(),
        field_type,
        offset_getter_fn_name,
    });

    Ok(())
}

fn is_vla(ty: &ast::Type<'_>) -> bool {
    matches!(
        ty,
        ast::Type::Array(ast::ArrayType {
            size: ast::ArraySize::Dynamic,
            ..
        })
    )
}

fn generate_vla_hook(
    hook: &Hook,
    struct_accum: &mut StructAccum,
    field_accum: &mut FieldAccum,
) -> Result<(), Error> {
    let field_name = &field_accum.field_name;
    let hook_fn = &hook.fn_path;
    let return_ty = &hook.return_ty;

    let raw_fn_name = format_ident!("{}_raw", field_name);

    let start_offset = match &struct_accum.last_offset_getter_fn_name {
        Some(prev) => quote! { self.#prev().byte },
        None => quote! { 0 },
    };

    field_accum.helper_fns = quote! {
        fn #raw_fn_name(&self) -> (#return_ty, usize) {
            #hook_fn(&self.data[#start_offset..])
        }
    };

    field_accum.field_getter = quote! {
        pub fn #field_name(&self) -> #return_ty {
            self.#raw_fn_name().0
        }
    };

    let len_expr = quote! {
        binparse::Len { byte: self.#raw_fn_name().1, bit: 0 }
    };
    field_accum.len = GeneratedLen::Dynamic(len_expr);
    field_accum.field_type = DoneFieldType::Other;

    Ok(())
}

fn generate_fixed_hook<'a>(
    hook: &Hook,
    ty: &ast::Type<'a>,
    done: &HashMap<&'a str, GeneratedStruct>,
    struct_accum: &mut StructAccum,
    field_accum: &mut FieldAccum,
    start_offset: GeneratedLen,
    endian: attr::Endian,
) -> Result<(), Error> {
    let info = type_::generate(ty, done, struct_accum, field_accum, start_offset, endian)?;

    field_accum.len = info.len;
    field_accum.field_type = DoneFieldType::Other;

    let field_name = &field_accum.field_name;
    let hook_fn = &hook.fn_path;
    let return_ty = &hook.return_ty;
    let field_getter_body = info.field_getter_body;

    field_accum.field_getter = quote! {
        #[allow(clippy::identity_op)]
        pub fn #field_name(&self) -> #return_ty {
            #hook_fn(#field_getter_body)
        }
    };

    Ok(())
}
