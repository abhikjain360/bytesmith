use std::collections::HashMap;

use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    GeneratedLen,
    attr::{self, Hook, ParsedAttrs},
    expr,
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
    #[error(transparent)]
    Expr(#[from] expr::Error),
    #[error("constant field literal width {width} is not supported")]
    ConstantTooWide { width: u8 },
    #[error("@range minimum {min} is greater than maximum {max}")]
    InvalidRange { min: usize, max: usize },
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
    let field_inherited = attrs.merge_inherited(struct_accum.inherited);
    let mut field_accum = FieldAccum::new(ast.name);

    let mut constant = None;
    match &ast.value {
        ast::FieldValue::Type(ty) => {
            if attrs.endian.is_some() {
                check_endian_applies(ty)?;
            }
            if attrs.bit_order.is_some() {
                check_bit_order_applies(ty)?;
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
                        field_inherited,
                    )?;
                }
                (None, _) => {
                    generate_plain(ty, done, struct_accum, &mut field_accum, field_inherited)?;
                }
            }
        }

        ast::FieldValue::Constraint(expr) => {
            let ast::Expr::Literal(ast::Literal::Int(lit)) = expr else {
                todo!("non-literal constraint fields")
            };
            let ty = constant_type(lit)?;
            if attrs.endian.is_some() {
                check_endian_applies(&ty)?;
            }
            if attrs.bit_order.is_some() {
                check_bit_order_applies(&ty)?;
            }
            generate_plain(&ty, done, struct_accum, &mut field_accum, field_inherited)?;
            constant = Some(lit.value);
        }
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

    generate_validations(ast, &attrs, constant, field_type, struct_accum)?;

    Ok(())
}

fn generate_validations<'a>(
    ast: &ast::Field<'a>,
    attrs: &ParsedAttrs<'a>,
    constant: Option<usize>,
    field_type: DoneFieldType,
    struct_accum: &mut StructAccum,
) -> Result<(), Error> {
    if constant.is_none() && attrs.check.is_none() && attrs.range.is_none() {
        return Ok(());
    }

    if !matches!(
        field_type,
        DoneFieldType::Primitive | DoneFieldType::BitField
    ) {
        return Err(attr::Error::ValidationOnNonNumeric.into());
    }

    let field_name = format_ident!("{}", ast.name);
    let field_path = format!("{}.{}", struct_accum.name, ast.name);
    let validation_error = quote! {
        binparse::ParseError::ValidationFailed {
            field: #field_path,
            actual: self.#field_name() as u128,
        }
    };

    let mut validations = TokenStream::new();

    if let Some(value) = constant {
        let expected = proc_macro2::Literal::u128_unsuffixed(value as u128);
        validations.extend(quote! {
            if self.#field_name() != #expected {
                return Err(#validation_error);
            }
        });
    }

    if let Some((min, max)) = &attrs.range {
        let min = expr::lower(min, expr::ExprType::Numeric, &struct_accum.done_fields)?;
        let max = expr::lower(max, expr::ExprType::Numeric, &struct_accum.done_fields)?;
        if let (Some(min_value), Some(max_value)) = (min.const_value, max.const_value)
            && min_value > max_value
        {
            return Err(Error::InvalidRange {
                min: min_value,
                max: max_value,
            });
        }
        let min_tokens = min.tokens;
        let max_tokens = max.tokens;
        validations.extend(quote! {
            if !((#min_tokens)..=(#max_tokens)).contains(&(self.#field_name() as usize)) {
                return Err(#validation_error);
            }
        });
    }

    if let Some(check) = &attrs.check {
        let check = expr::lower(check, expr::ExprType::Bool, &struct_accum.done_fields)?;
        let check_tokens = check.tokens;
        validations.extend(quote! {
            if !#check_tokens {
                return Err(#validation_error);
            }
        });
    }

    let validate_fn_name = format_ident!("{}_validate", ast.name);
    struct_accum.functions.extend(quote! {
        #[allow(clippy::unnecessary_cast)]
        fn #validate_fn_name(&self) -> Result<(), binparse::ParseError> {
            #validations
            Ok(())
        }
    });
    struct_accum.parse_checks.extend(quote! {
        me.#validate_fn_name()?;
    });

    Ok(())
}

fn generate_plain<'a>(
    ty: &ast::Type<'a>,
    done: &HashMap<&'a str, GeneratedStruct>,
    struct_accum: &mut StructAccum,
    field_accum: &mut FieldAccum,
    inherited: attr::Inherited,
) -> Result<(), Error> {
    let start_offset = struct_accum.offset.clone();
    let info = type_::generate(ty, done, struct_accum, field_accum, start_offset, inherited)?;

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

    Ok(())
}

fn constant_type(lit: &ast::IntLiteral) -> Result<ast::Type<'static>, Error> {
    match lit.ty {
        ast::IntType::Binary => match lit.width {
            1..=8 => Ok(ast::Type::BitField(lit.width)),
            _ => Err(Error::ConstantTooWide { width: lit.width }),
        },
        ast::IntType::Hex => match usize::from(lit.width).div_ceil(2) {
            1 => Ok(ast::Type::Primitive(ast::Primitive::U8)),
            2 => Ok(ast::Type::Primitive(ast::Primitive::U16)),
            3..=4 => Ok(ast::Type::Primitive(ast::Primitive::U32)),
            5..=8 => Ok(ast::Type::Primitive(ast::Primitive::U64)),
            9..=16 => Ok(ast::Type::Primitive(ast::Primitive::U128)),
            _ => Err(Error::ConstantTooWide { width: lit.width }),
        },
        ast::IntType::Decimal => {
            let primitive = if lit.value <= usize::from(u8::MAX) {
                ast::Primitive::U8
            } else if lit.value <= usize::from(u16::MAX) {
                ast::Primitive::U16
            } else if u32::try_from(lit.value).is_ok() {
                ast::Primitive::U32
            } else {
                ast::Primitive::U64
            };
            Ok(ast::Type::Primitive(primitive))
        }
    }
}

fn is_vla(ty: &ast::Type<'_>) -> bool {
    matches!(ty, ast::Type::Array(ast::ArrayType { size: None, .. }))
}

fn check_endian_applies(ty: &ast::Type<'_>) -> Result<(), attr::Error> {
    match ty {
        ast::Type::Primitive(ast::Primitive::U8 | ast::Primitive::I8) => {
            Err(attr::Error::EndianOnSingleByte)
        }
        ast::Type::BitField(_) => Err(attr::Error::EndianOnBitfield),
        ast::Type::StructRef(_) => Err(attr::Error::EndianOnStructRef),
        ast::Type::Array(ast::ArrayType { elem_ty, .. }) => match elem_ty {
            ast::ArrayElemType::Primitive(ast::Primitive::U8 | ast::Primitive::I8) => {
                Err(attr::Error::EndianOnSingleByte)
            }
            ast::ArrayElemType::BitField(_) => Err(attr::Error::EndianOnBitfield),
            ast::ArrayElemType::StructRef(_) => Err(attr::Error::EndianOnStructRef),
            ast::ArrayElemType::Primitive(_) => Ok(()),
        },
        _ => Ok(()),
    }
}

fn check_bit_order_applies(ty: &ast::Type<'_>) -> Result<(), attr::Error> {
    match ty {
        ast::Type::BitField(_)
        | ast::Type::Array(ast::ArrayType {
            elem_ty: ast::ArrayElemType::BitField(_),
            ..
        })
        | ast::Type::Concat(_)
        | ast::Type::Union(_) => Ok(()),
        _ => Err(attr::Error::BitOrderOnNonBitfield),
    }
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
    inherited: attr::Inherited,
) -> Result<(), Error> {
    let info = type_::generate(ty, done, struct_accum, field_accum, start_offset, inherited)?;

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
