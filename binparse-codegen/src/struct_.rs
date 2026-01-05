use std::collections::HashMap;

use binparse_dsl as ast;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{ToTokens, quote};

use crate::{ExprContext, Len};

#[derive(Clone)]
pub(crate) struct StructCtx<'a> {
    pub(crate) origin: ast::Struct<'a>,
    pub(crate) static_offset: Option<Len>,
    pub(crate) dynamic_offset: TokenStream,
    pub(crate) extra_fields: Vec<(Ident, TokenStream)>,
    pub(crate) parse_stmts: TokenStream,
    pub(crate) done: &'a HashMap<&'a str, GeneratedStruct>,
    pub(crate) endian: Endian,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Endian {
    Big,
    Little,
}

impl Default for Endian {
    fn default() -> Self {
        Endian::Big
    }
}

pub(crate) struct GeneratedStruct {
    pub(crate) len: Option<Len>,
    pub(crate) tokens: TokenStream,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid attribute argument")]
    InvalidAttribute,
    #[error("Missing dependencies for {0}")]
    MissingDependency(String),
}

fn to_ident(name: &str) -> Ident {
    match name {
        "const" | "enum" | "extern" | "fn" | "impl" | "let" | "mod" | "pub" | "struct"
        | "trait" | "type" | "use" | "where" | "mut" | "match" | "if" | "else" | "return"
        | "loop" | "while" | "for" | "in" | "continue" | "break" | "crate" | "super" | "self"
        | "Self" | "unsafe" | "async" | "await" | "dyn" | "abstract" | "become" | "box" | "do"
        | "final" | "macro" | "override" | "priv" | "typeof" | "unsized" | "virtual" | "yield"
        | "try" => Ident::new_raw(name, Span::call_site()),
        _ => Ident::new(name, Span::call_site()),
    }
}

impl<'a> StructCtx<'a> {
    pub(crate) fn new(
        origin: ast::Struct<'a>,
        done: &'a HashMap<&'a str, GeneratedStruct>,
    ) -> Self {
        let mut endian = Endian::Big;
        for attr in &origin.attributes {
            if attr.name == "endian" {
                if let Some(ast::AttributeArg::String(s)) = attr.args.first() {
                    if *s == "little" {
                        endian = Endian::Little;
                    }
                } else if let Some(ast::AttributeArg::Math(ast::MathExpr::Atom(
                    ast::NumericAtom::Variable(v),
                ))) = attr.args.first()
                {
                    if v.len() == 1 && v[0] == "little" {
                        endian = Endian::Little;
                    }
                }
            }
        }

        Self {
            origin,
            static_offset: Some(Len { byte: 0, bit: 0 }),
            dynamic_offset: quote! { 0 },
            extra_fields: Vec::new(),
            parse_stmts: TokenStream::new(),
            done,
            endian,
        }
    }

    pub(crate) fn generate(mut self) -> Result<GeneratedStruct, Error> {
        let struct_name = to_ident(self.origin.name);
        let mut accessors = TokenStream::new();
        let mut extra_types = TokenStream::new();

        self.parse_stmts.extend(quote! {
            let mut cursor = 0;
        });

        let items = self.origin.items.clone();
        for item in &items {
            match item {
                ast::StructItem::Field(field) => {
                    self.generate_field(
                        field,
                        &mut accessors,
                        &mut extra_types,
                        None,
                        None,
                        false,
                    )?;
                }
                ast::StructItem::Conditional(cond) => {
                    self.generate_conditional(cond, &mut accessors, &mut extra_types)?;
                }
            }
        }

        let consumed_logic = if let Some(len) = self.static_offset {
            let bytes = len.byte + if len.bit > 0 { 1 } else { 0 };
            let lit = proc_macro2::Literal::usize_unsuffixed(bytes);
            quote! { #lit }
        } else {
            quote! { self._consumed }
        };

        if self.static_offset.is_none() {
            self.extra_fields
                .push((Ident::new("_consumed", Span::call_site()), quote! { usize }));
            self.parse_stmts.extend(quote! {
                let _consumed = cursor;
            });
        }

        let extra_field_defs = self.extra_fields.iter().map(|(name, ty)| {
            quote! { #name: #ty, }
        });

        let extra_field_inits = self.extra_fields.iter().map(|(name, _)| {
            quote! { #name, }
        });

        let parse_stmts = &self.parse_stmts;

        Ok(GeneratedStruct {
            len: self.static_offset,
            tokens: quote! {
                #extra_types

                #[derive(Debug, Clone)]
                pub struct #struct_name<'a> {
                    data: &'a [u8],
                    #(#extra_field_defs)*
                }

                impl<'a> #struct_name<'a> {
                    pub fn parse(data: &'a [u8]) -> Result<Self, binparse::Error> {
                        #parse_stmts
                        Ok(Self {
                            data,
                            #(#extra_field_inits)*
                        })
                    }

                    pub fn consumed(&self) -> usize {
                        #consumed_logic
                    }

                    #accessors
                }
            },
        })
    }

    fn collect_fields<'b>(items: &'b [ast::StructItem<'a>], fields: &mut Vec<&'b ast::Field<'a>>) {
        for item in items {
            match item {
                ast::StructItem::Field(f) => fields.push(f),
                ast::StructItem::Conditional(c) => {
                    Self::collect_fields(&c.then_branch, fields);
                    if let Some(else_branch) = &c.else_branch {
                        Self::collect_fields(else_branch, fields);
                    }
                }
            }
        }
    }

    fn generate_conditional(
        &mut self,
        cond: &ast::Conditional<'a>,
        accessors: &mut TokenStream,
        extra_types: &mut TokenStream,
    ) -> Result<(), Error> {
        self.static_offset = None;
        self.dynamic_offset = quote! { cursor };

        let mut fields = Vec::new();
        Self::collect_fields(&cond.then_branch, &mut fields);
        if let Some(else_branch) = &cond.else_branch {
            Self::collect_fields(else_branch, &mut fields);
        }

        for field in fields {
            let offset_ident = Ident::new(&format!("_{}_offset", field.name), Span::call_site());
            if !self.extra_fields.iter().any(|(n, _)| n == &offset_ident) {
                self.extra_fields
                    .push((offset_ident.clone(), quote! { Option<usize> }));
                self.parse_stmts
                    .extend(quote! { let mut #offset_ident = None; });
            }

            if let ast::FieldValue::Type(ty) = &field.value {
                match ty {
                    ast::Type::Array(_) => {
                        let count_ident =
                            Ident::new(&format!("_{}_count", field.name), Span::call_site());
                        if !self.extra_fields.iter().any(|(n, _)| n == &count_ident) {
                            self.extra_fields
                                .push((count_ident.clone(), quote! { Option<usize> }));
                            self.parse_stmts
                                .extend(quote! { let mut #count_ident = None; });
                        }
                    }
                    ast::Type::StructRef(_) => {
                        let len_ident =
                            Ident::new(&format!("_{}_len", field.name), Span::call_site());
                        if !self.extra_fields.iter().any(|(n, _)| n == &len_ident) {
                            self.extra_fields
                                .push((len_ident.clone(), quote! { Option<usize> }));
                            self.parse_stmts
                                .extend(quote! { let mut #len_ident = None; });
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut then_stmts = TokenStream::new();
        let saved_stmts = std::mem::replace(&mut self.parse_stmts, then_stmts);

        for item in &cond.then_branch {
            match item {
                ast::StructItem::Field(f) => {
                    self.generate_field(f, accessors, extra_types, None, None, true)?;
                }
                ast::StructItem::Conditional(c) => {
                    self.generate_conditional(c, accessors, extra_types)?;
                }
            }
        }

        let then_stmts = std::mem::replace(&mut self.parse_stmts, saved_stmts);

        let mut else_stmts = TokenStream::new();
        if let Some(else_branch) = &cond.else_branch {
            let saved_stmts = std::mem::replace(&mut self.parse_stmts, else_stmts);
            for item in else_branch {
                match item {
                    ast::StructItem::Field(f) => {
                        self.generate_field(f, accessors, extra_types, None, None, true)?;
                    }
                    ast::StructItem::Conditional(c) => {
                        self.generate_conditional(c, accessors, extra_types)?;
                    }
                }
            }
            else_stmts = std::mem::replace(&mut self.parse_stmts, saved_stmts);
        }

        let cond_expr = crate::generate_bool_expr(&cond.condition, &quote! {}, ExprContext::Parse);

        if cond.else_branch.is_some() {
            self.parse_stmts.extend(quote! {
                if #cond_expr {
                    #then_stmts
                } else {
                    #else_stmts
                }
            });
        } else {
            self.parse_stmts.extend(quote! {
                if #cond_expr {
                    #then_stmts
                }
            });
        }

        Ok(())
    }

    fn generate_field(
        &mut self,
        field: &ast::Field<'a>,
        accessors: &mut TokenStream,
        extra_types: &mut TokenStream,
        check_condition: Option<&TokenStream>,
        accessor_condition: Option<&TokenStream>,
        in_conditional: bool,
    ) -> Result<(), Error> {
        let name_ident = Ident::new_raw(field.name, Span::call_site());
        let mut endian = self.endian;

        let mut skip = false;
        let mut align = None;
        let mut opaque = false;
        let mut greedy = false;
        let mut until = None;
        let mut transform = None;
        let mut parse_with = None;
        let mut len_limit = None;

        for attr in &field.attributes {
            match attr.name {
                "endian" | "bit_order" => {
                    if let Some(ast::AttributeArg::String(s)) = attr.args.first() {
                        if *s == "little" || *s == "lsb" {
                            endian = Endian::Little;
                        } else {
                            endian = Endian::Big;
                        }
                    } else if let Some(ast::AttributeArg::Math(ast::MathExpr::Atom(
                        ast::NumericAtom::Variable(v),
                    ))) = attr.args.first()
                    {
                        if v.len() == 1 && (v[0] == "little" || v[0] == "lsb") {
                            endian = Endian::Little;
                        } else {
                            endian = Endian::Big;
                        }
                    }
                }
                "skip" => skip = true,
                "align" => {
                    if let Some(ast::AttributeArg::Math(expr)) = attr.args.first() {
                        align = Some(expr);
                    }
                }
                "opaque" => opaque = true,
                "greedy" => greedy = true,
                "until" => {
                    if let Some(ast::AttributeArg::Math(expr)) = attr.args.first() {
                        until = Some(expr);
                    }
                }
                "transform" => {
                    if let Some(ast::AttributeArg::String(s)) = attr.args.first() {
                        transform = Some(s.clone());
                    }
                }
                "parse_with" => {
                    if let Some(ast::AttributeArg::String(s)) = attr.args.first() {
                        parse_with = Some(s.clone());
                    }
                }
                "len" | "limit" => {
                    if let Some(ast::AttributeArg::Math(expr)) = attr.args.first() {
                        len_limit = Some(expr);
                    }
                }
                _ => {}
            }
        }

        if let Some(align_expr) = align {
            let align_val = crate::generate_math_expr(align_expr, &quote! {}, ExprContext::Parse);
            self.parse_stmts.extend(quote! {
                if cursor % (#align_val as usize) != 0 {
                    return Err(binparse::Error::InvalidValue);
                }
            });
        }

        let start_offset_expr = if let Some(offset) = self.static_offset {
            let bytes = offset.byte;
            quote! { #bytes }
        } else {
            self.dynamic_offset.clone()
        };

        match &field.value {
            ast::FieldValue::Type(ty) => match ty {
                ast::Type::Primitive(p) => {
                    let (len, type_token, _) = crate::match_primitive(p);

                    if let ast::Primitive::BitField(w) = p {
                        let width = *w as usize;
                        let _width_lit = proc_macro2::Literal::usize_unsuffixed(width);

                        let mut current_bit = 0;
                        if let Some(so) = self.static_offset {
                            current_bit = so.bit;
                        }

                        let total_bits = current_bit + width;
                        let added_bytes = total_bits / 8;
                        let new_bit = total_bits % 8;

                        let bytes_needed = (total_bits + 7) / 8;
                        let bytes_needed_lit = proc_macro2::Literal::usize_unsuffixed(bytes_needed);

                        if current_bit == 0 {
                            self.parse_stmts.extend(quote! {
                                if data.len() < cursor + #bytes_needed_lit {
                                    return Err(binparse::Error::UnexpectedEof);
                                }
                            });
                        }

                        let read_val = if total_bits <= 8 {
                            let shift = 8 - total_bits;
                            let mask = (1u16 << width) - 1;
                            quote! {
                                ((data[cursor] >> #shift) & #mask as u8)
                            }
                        } else {
                            quote! { 0 }
                        };

                        self.parse_stmts.extend(quote! {
                            let #name_ident = #read_val;
                        });

                        if let Some(so) = self.static_offset {
                            self.static_offset = Some(Len {
                                byte: so.byte + added_bytes,
                                bit: new_bit,
                            });
                        }

                        if !skip {
                            accessors.extend(quote! {
                                pub fn #name_ident(&self) -> (#type_token, usize) {
                                    (0, 0)
                                }
                            });
                        }
                    } else {
                        let width = len.byte;
                        let width_lit = proc_macro2::Literal::usize_unsuffixed(width);

                        self.parse_stmts.extend(quote! {
                            if data.len() < cursor + #width_lit {
                                return Err(binparse::Error::UnexpectedEof);
                            }
                        });

                        let read_expr = match endian {
                            Endian::Big => match p {
                                ast::Primitive::U8 => quote! { data[cursor] },
                                ast::Primitive::U16 => {
                                    quote! { u16::from_be_bytes(data[cursor..cursor+2].try_into().unwrap()) }
                                }
                                ast::Primitive::U32 => {
                                    quote! { u32::from_be_bytes(data[cursor..cursor+4].try_into().unwrap()) }
                                }
                                ast::Primitive::U64 => {
                                    quote! { u64::from_be_bytes(data[cursor..cursor+8].try_into().unwrap()) }
                                }
                                _ => quote! { 0 },
                            },
                            Endian::Little => match p {
                                ast::Primitive::U8 => quote! { data[cursor] },
                                ast::Primitive::U16 => {
                                    quote! { u16::from_le_bytes(data[cursor..cursor+2].try_into().unwrap()) }
                                }
                                ast::Primitive::U32 => {
                                    quote! { u32::from_le_bytes(data[cursor..cursor+4].try_into().unwrap()) }
                                }
                                ast::Primitive::U64 => {
                                    quote! { u64::from_le_bytes(data[cursor..cursor+8].try_into().unwrap()) }
                                }
                                _ => quote! { 0 },
                            },
                        };

                        self.parse_stmts.extend(quote! {
                            let #name_ident = #read_expr;
                            cursor += #width_lit;
                        });

                        if let Some(so) = self.static_offset {
                            self.static_offset = Some(so + len);
                        }

                        if !skip {
                            let accessor_offset = if self.static_offset.is_some() {
                                start_offset_expr.clone()
                            } else {
                                let offset_field_ident = Ident::new(
                                    &format!("_{}_offset", field.name),
                                    Span::call_site(),
                                );
                                if in_conditional {
                                    let val_expr = quote! { cursor - #width_lit };
                                    self.parse_stmts.extend(quote! {
                                        #offset_field_ident = Some(#val_expr);
                                    });
                                    quote! { self.#offset_field_ident }
                                } else {
                                    if self
                                        .extra_fields
                                        .iter()
                                        .find(|(n, _)| n == &offset_field_ident)
                                        .is_none()
                                    {
                                        self.extra_fields
                                            .push((offset_field_ident.clone(), quote! { usize }));
                                        self.parse_stmts.extend(quote! {
                                            let #offset_field_ident = cursor - #width_lit;
                                        });
                                    }
                                    quote! { self.#offset_field_ident }
                                }
                            };

                            let access_expr = match endian {
                                Endian::Big => match width {
                                    1 => quote! { self.data[offset] as #type_token },
                                    2 => {
                                        quote! { u16::from_be_bytes(self.data[offset..offset+2].try_into().unwrap()) as #type_token }
                                    }
                                    4 => {
                                        quote! { u32::from_be_bytes(self.data[offset..offset+4].try_into().unwrap()) as #type_token }
                                    }
                                    8 => {
                                        quote! { u64::from_be_bytes(self.data[offset..offset+8].try_into().unwrap()) as #type_token }
                                    }
                                    _ => quote! { 0 },
                                },
                                Endian::Little => match width {
                                    1 => quote! { self.data[offset] as #type_token },
                                    2 => {
                                        quote! { u16::from_le_bytes(self.data[offset..offset+2].try_into().unwrap()) as #type_token }
                                    }
                                    4 => {
                                        quote! { u32::from_le_bytes(self.data[offset..offset+4].try_into().unwrap()) as #type_token }
                                    }
                                    8 => {
                                        quote! { u64::from_le_bytes(self.data[offset..offset+8].try_into().unwrap()) as #type_token }
                                    }
                                    _ => quote! { 0 },
                                },
                            };

                            if in_conditional {
                                accessors.extend(quote! {
                                    pub fn #name_ident(&self) -> Option<(#type_token, usize)> {
                                        let offset = #accessor_offset?;
                                        let val = #access_expr;
                                        Some((val, offset))
                                    }
                                });
                            } else {
                                accessors.extend(quote! {
                                    pub fn #name_ident(&self) -> (#type_token, usize) {
                                        let offset = #accessor_offset;
                                        let val = #access_expr;
                                        (val, offset)
                                    }
                                });
                            }
                        }
                    }
                }
                ast::Type::Array(arr) => {
                    let mut is_fixed_size = false;
                    let mut elem_size = 0;
                    let mut is_bitfield = false;
                    let mut bit_width = 0;

                    if let ast::Type::Primitive(p) = &arr.elem_ty {
                        let (len, _, _) = crate::match_primitive(p);
                        if let ast::Primitive::BitField(w) = p {
                            is_bitfield = true;
                            bit_width = *w as usize;
                        } else if len.bit == 0 {
                            is_fixed_size = true;
                            elem_size = len.byte;
                        }
                    }

                    let start_cursor = quote! { cursor };

                    if is_bitfield {
                        let start_bit_offset = if let Some(so) = self.static_offset {
                            so.bit
                        } else {
                            0
                        };
                        let start_bit_lit =
                            proc_macro2::Literal::usize_unsuffixed(start_bit_offset);
                        let bit_width_lit = proc_macro2::Literal::usize_unsuffixed(bit_width);
                        let mut count_lit = quote! { 0 };

                        if let Some(expr) = &arr.size_expr {
                            let count_expr =
                                crate::generate_math_expr(expr, &quote! {}, ExprContext::Parse);
                            count_lit = quote! { #count_expr as usize };
                            self.parse_stmts.extend(quote! {
                                let count = #count_expr as usize;
                                let total_bits = count * #bit_width_lit;
                                let size = (total_bits + 7) / 8;
                                if data.len() < cursor + size {
                                    return Err(binparse::Error::UnexpectedEof);
                                }
                                cursor += size;
                            });
                        } else {
                            self.parse_stmts.extend(quote! {});
                        }

                        self.static_offset = None;
                        self.dynamic_offset = quote! { cursor };

                        let offset_field =
                            Ident::new(&format!("_{}_offset", field.name), Span::call_site());
                        let count_field =
                            Ident::new(&format!("_{}_count", field.name), Span::call_site());

                        if in_conditional {
                            self.parse_stmts.extend(quote! {
                                #offset_field = Some(#start_cursor);
                                #count_field = Some(#count_lit);
                            });
                        } else {
                            self.extra_fields
                                .push((offset_field.clone(), quote! { usize }));
                            self.extra_fields
                                .push((count_field.clone(), quote! { usize }));

                            self.parse_stmts.extend(quote! {
                                let #offset_field = #start_cursor;
                                let #count_field = #count_lit;
                            });
                        }

                        let (_elem_len, elem_type, _) =
                            if let ast::Type::Primitive(p) = &arr.elem_ty {
                                crate::match_primitive(p)
                            } else {
                                unreachable!()
                            };

                        if in_conditional {
                            accessors.extend(quote! {
                                pub fn #name_ident(&self) -> Option<impl Iterator<Item = #elem_type> + '_> {
                                    let start_byte = self.#offset_field?;
                                    let count = self.#count_field?;
                                    let width = #bit_width_lit;
                                    let start_bit = #start_bit_lit;
                                    Some((0..count).map(move |i| {
                                        let total_bit_offset = start_bit + i * width;
                                        let byte_off = total_bit_offset / 8;
                                        let bit_off = total_bit_offset % 8;
                                        let offset = start_byte + byte_off;
                                        let shift = 8 - bit_off - width;
                                        let mask = ((1u16 << width) - 1) as u8;
                                        ((self.data[offset] >> shift) & mask) as #elem_type
                                    }))
                                }
                            });
                        } else {
                            accessors.extend(quote! {
                                pub fn #name_ident(&self) -> impl Iterator<Item = #elem_type> + '_ {
                                    let start_byte = self.#offset_field;
                                    let count = self.#count_field;
                                    let width = #bit_width_lit;
                                    let start_bit = #start_bit_lit;
                                    (0..count).map(move |i| {
                                        let total_bit_offset = start_bit + i * width;
                                        let byte_off = total_bit_offset / 8;
                                        let bit_off = total_bit_offset % 8;
                                        let offset = start_byte + byte_off;
                                        let shift = 8 - bit_off - width;
                                        let mask = ((1u16 << width) - 1) as u8;
                                        ((self.data[offset] >> shift) & mask) as #elem_type
                                    })
                                }
                            });
                        }
                    } else if is_fixed_size {
                        let elem_size_lit = proc_macro2::Literal::usize_unsuffixed(elem_size);

                        if let Some(expr) = &arr.size_expr {
                            let count_expr =
                                crate::generate_math_expr(expr, &quote! {}, ExprContext::Parse);

                            if let Some(limit_expr) = len_limit {
                                let limit_val = crate::generate_math_expr(
                                    limit_expr,
                                    &quote! {},
                                    ExprContext::Parse,
                                );
                                self.parse_stmts.extend(quote! {
                                    let limit = #limit_val as usize;
                                    if data.len() < cursor + limit {
                                        return Err(binparse::Error::UnexpectedEof);
                                    }
                                    let count = #count_expr as usize;
                                    let size = count * #elem_size_lit;
                                    if size > limit {
                                        return Err(binparse::Error::BadLength);
                                    }
                                    cursor += limit;
                                });
                            } else {
                                self.parse_stmts.extend(quote! {
                                    let count = #count_expr as usize;
                                    let size = count * #elem_size_lit;
                                    if data.len() < cursor + size {
                                        return Err(binparse::Error::UnexpectedEof);
                                    }
                                    cursor += size;
                                });
                            }

                            self.parse_stmts.extend(quote! {
                                let count = #count_expr as usize;
                            });
                        } else {
                            if let Some(limit_expr) = len_limit {
                                let limit_val = crate::generate_math_expr(
                                    limit_expr,
                                    &quote! {},
                                    ExprContext::Parse,
                                );
                                self.parse_stmts.extend(quote! {
                                    let limit = #limit_val as usize;
                                    if data.len() < cursor + limit {
                                        return Err(binparse::Error::UnexpectedEof);
                                    }
                                    let count = limit / #elem_size_lit;
                                    cursor += limit;
                                });
                            } else if greedy {
                                self.parse_stmts.extend(quote! {
                                    let remaining = data.len() - cursor;
                                    let count = remaining / #elem_size_lit;
                                    let size = count * #elem_size_lit;
                                    cursor += size;
                                });
                            } else {
                                self.parse_stmts.extend(quote! {
                                    let count = 0;
                                });
                            }
                        }

                        self.static_offset = None;
                        self.dynamic_offset = quote! { cursor };

                        let offset_field =
                            Ident::new(&format!("_{}_offset", field.name), Span::call_site());
                        let count_field =
                            Ident::new(&format!("_{}_count", field.name), Span::call_site());

                        if in_conditional {
                            self.parse_stmts.extend(quote! {
                                #offset_field = Some(#start_cursor);
                                #count_field = Some(count);
                            });
                        } else {
                            self.extra_fields
                                .push((offset_field.clone(), quote! { usize }));
                            self.extra_fields
                                .push((count_field.clone(), quote! { usize }));

                            self.parse_stmts.extend(quote! {
                                let #offset_field = #start_cursor;
                                let #count_field = count;
                            });
                        }

                        let (_elem_len, elem_type, _) =
                            if let ast::Type::Primitive(p) = &arr.elem_ty {
                                crate::match_primitive(p)
                            } else {
                                unreachable!()
                            };

                        let access_expr = match endian {
                            Endian::Big => match elem_size {
                                1 => quote! { self.data[offset] as #elem_type },
                                2 => {
                                    quote! { u16::from_be_bytes(self.data[offset..offset+2].try_into().unwrap()) as #elem_type }
                                }
                                _ => quote! { 0 },
                            },
                            Endian::Little => match elem_size {
                                1 => quote! { self.data[offset] as #elem_type },
                                2 => {
                                    quote! { u16::from_le_bytes(self.data[offset..offset+2].try_into().unwrap()) as #elem_type }
                                }
                                _ => quote! { 0 },
                            },
                        };

                        if in_conditional {
                            accessors.extend(quote! {
                                pub fn #name_ident(&self) -> Option<impl Iterator<Item = #elem_type> + '_> {
                                    let start = self.#offset_field?;
                                    let count = self.#count_field?;
                                    let width = #elem_size_lit;
                                    Some((0..count).map(move |i| {
                                        let offset = start + i * width;
                                        #access_expr
                                    }))
                                }
                            });
                        } else {
                            accessors.extend(quote! {
                                pub fn #name_ident(&self) -> impl Iterator<Item = #elem_type> + '_ {
                                    let start = self.#offset_field;
                                    let count = self.#count_field;
                                    let width = #elem_size_lit;
                                    (0..count).map(move |i| {
                                        let offset = start + i * width;
                                        #access_expr
                                    })
                                }
                            });
                        }
                    }
                }
                ast::Type::Concat(fields) => {
                    let mut total_bits = 0;
                    let mut field_widths = Vec::new();

                    for field in fields {
                        if let ast::FieldValue::Type(ast::Type::Primitive(
                            ast::Primitive::BitField(w),
                        )) = &field.value
                        {
                            total_bits += *w as usize;
                            field_widths.push(*w as usize);
                        } else {
                            if let ast::FieldValue::Type(ast::Type::Primitive(p)) = &field.value {
                                let (len, _, _) = crate::match_primitive(p);
                                let w = len.byte * 8 + len.bit;
                                total_bits += w;
                                field_widths.push(w);
                            }
                        }
                    }

                    let return_ty = if total_bits <= 8 {
                        quote! { u8 }
                    } else if total_bits <= 16 {
                        quote! { u16 }
                    } else if total_bits <= 32 {
                        quote! { u32 }
                    } else if total_bits <= 64 {
                        quote! { u64 }
                    } else {
                        quote! { u128 }
                    };

                    let start_bit = if let Some(so) = self.static_offset {
                        so.bit
                    } else {
                        0
                    };
                    let bytes_needed = (start_bit + total_bits + 7) / 8;
                    let bytes_needed_lit = proc_macro2::Literal::usize_unsuffixed(bytes_needed);

                    self.parse_stmts.extend(quote! {
                        if data.len() < cursor + #bytes_needed_lit {
                            return Err(binparse::Error::UnexpectedEof);
                        }
                    });

                    let mut read_stmts = TokenStream::new();
                    read_stmts.extend(quote! { let mut val: #return_ty = 0; });

                    let mut current_bit = start_bit;

                    for width in &field_widths {
                        let w = *width;
                        let w_lit = proc_macro2::Literal::usize_unsuffixed(w);
                        let byte_off = current_bit / 8;
                        let bit_off = current_bit % 8;

                        if bit_off + w <= 8 {
                            let shift = 8 - bit_off - w;
                            let mask = ((1u16 << w) - 1) as u8;
                            let byte_idx = byte_off;
                            let byte_idx_lit = proc_macro2::Literal::usize_unsuffixed(byte_idx);
                            read_stmts.extend(quote! {
                                    val = (val << #w_lit) | (((data[cursor + #byte_idx_lit] >> #shift) & #mask) as #return_ty);
                                });
                        } else {
                            let part1_bits = 8 - bit_off;
                            let part2_bits = w - part1_bits;

                            let shift1 = 0;
                            let mask1 = ((1u16 << part1_bits) - 1) as u8;
                            let byte_idx1 = byte_off;
                            let byte_idx1_lit = proc_macro2::Literal::usize_unsuffixed(byte_idx1);

                            let shift2 = 8 - part2_bits;
                            let mask2 = ((1u16 << part2_bits) - 1) as u8;
                            let byte_idx2 = byte_off + 1;
                            let byte_idx2_lit = proc_macro2::Literal::usize_unsuffixed(byte_idx2);

                            let part1_lit = proc_macro2::Literal::usize_unsuffixed(part1_bits);
                            let part2_lit = proc_macro2::Literal::usize_unsuffixed(part2_bits);

                            read_stmts.extend(quote! {
                                    let p1 = ((data[cursor + #byte_idx1_lit] >> #shift1) & #mask1) as #return_ty;
                                    let p2 = ((data[cursor + #byte_idx2_lit] >> #shift2) & #mask2) as #return_ty;
                                    val = (val << #part1_lit) | p1;
                                    val = (val << #part2_lit) | p2;
                                });
                        }

                        current_bit += w;
                    }

                    self.parse_stmts.extend(read_stmts);
                    self.parse_stmts.extend(quote! {
                        let #name_ident = val;
                    });

                    if let Some(so) = self.static_offset {
                        let current_abs_bits = so.byte * 8 + so.bit;
                        let new_abs_bits = current_abs_bits + total_bits;
                        self.static_offset = Some(Len {
                            byte: new_abs_bits / 8,
                            bit: new_abs_bits % 8,
                        });
                    } else {
                        self.static_offset = Some(Len {
                            byte: total_bits / 8,
                            bit: total_bits % 8,
                        });
                    }

                    if !skip {
                        let accessor_offset = if self.static_offset.is_some() {
                            start_offset_expr.clone()
                        } else {
                            quote! { 0 }
                        };

                        let mut read_stmts = TokenStream::new();
                        read_stmts.extend(quote! { let mut val: #return_ty = 0; });

                        let mut acc_current_bit = start_bit;

                        for width in &field_widths {
                            let w = *width;
                            let w_lit = proc_macro2::Literal::usize_unsuffixed(w);
                            let byte_off = acc_current_bit / 8;
                            let bit_off = acc_current_bit % 8;
                            let byte_off_lit = proc_macro2::Literal::usize_unsuffixed(byte_off);

                            if bit_off + w <= 8 {
                                let shift = 8 - bit_off - w;
                                let mask = ((1u16 << w) - 1) as u8;
                                read_stmts.extend(quote! {
                                        val = (val << #w_lit) | (((self.data[offset + #byte_off_lit] >> #shift) & #mask) as #return_ty);
                                    });
                            } else {
                                let part1_bits = 8 - bit_off;
                                let part2_bits = w - part1_bits;
                                let shift1 = 0;
                                let mask1 = ((1u16 << part1_bits) - 1) as u8;
                                let shift2 = 8 - part2_bits;
                                let mask2 = ((1u16 << part2_bits) - 1) as u8;
                                let byte_off2_lit =
                                    proc_macro2::Literal::usize_unsuffixed(byte_off + 1);
                                let part1_lit = proc_macro2::Literal::usize_unsuffixed(part1_bits);
                                let part2_lit = proc_macro2::Literal::usize_unsuffixed(part2_bits);

                                read_stmts.extend(quote! {
                                        let p1 = ((self.data[offset + #byte_off_lit] >> #shift1) & #mask1) as #return_ty;
                                        let p2 = ((self.data[offset + #byte_off2_lit] >> #shift2) & #mask2) as #return_ty;
                                        val = (val << #part1_lit) | p1;
                                        val = (val << #part2_lit) | p2;
                                    });
                            }
                            acc_current_bit += w;
                        }

                        if in_conditional {
                            accessors.extend(quote! {
                                pub fn #name_ident(&self) -> Option<(#return_ty, usize)> {
                                    let offset = #accessor_offset?; // This might be "0" expression if static_offset logic failed
                                    // But dynamic_offset should be cursor.
                                    // Concat uses start_offset_expr.
                                    // If in_conditional, static_offset is None.
                                    // start_offset_expr is dynamic_offset (cursor).
                                    // But Concat doesn't store offset?
                                    // It uses offset calculation.
                                    // Wait, Concat logic uses "offset" variable in read_stmts.
                                    // So we need "offset".
                                    #read_stmts
                                    Some((val, offset))
                                }
                            });
                        } else {
                            accessors.extend(quote! {
                                pub fn #name_ident(&self) -> (#return_ty, usize) {
                                    let offset = #accessor_offset;
                                    #read_stmts
                                    (val, offset)
                                }
                            });
                        }
                    }
                }
                ast::Type::StructRef(path) => {
                    let struct_name = path.last().unwrap();
                    let struct_ident = Ident::new_raw(struct_name, Span::call_site());

                    let start_cursor = quote! { cursor };

                    if let Some(limit_expr) = len_limit {
                        let limit_val =
                            crate::generate_math_expr(limit_expr, &quote! {}, ExprContext::Parse);
                        self.parse_stmts.extend(quote! {
                            let limit = #limit_val as usize;
                            if data.len() < cursor + limit {
                                return Err(binparse::Error::UnexpectedEof);
                            }
                            let slice = &data[cursor..cursor+limit];
                            cursor += limit;
                        });
                    } else {
                        self.parse_stmts.extend(quote! {
                            let slice = &data[cursor..];
                            let #name_ident = #struct_ident::parse(slice)?;
                            cursor += #name_ident.consumed();
                        });
                    }

                    let offset_field =
                        Ident::new(&format!("_{}_offset", field.name), Span::call_site());
                    let len_field = Ident::new(&format!("_{}_len", field.name), Span::call_site());

                    if in_conditional {
                        self.parse_stmts.extend(quote! {
                            #offset_field = Some(#start_cursor);
                            #len_field = Some(#name_ident.consumed());
                        });
                    } else {
                        self.extra_fields
                            .push((offset_field.clone(), quote! { usize }));
                        self.extra_fields
                            .push((len_field.clone(), quote! { usize }));

                        if let Some(_) = len_limit {
                            self.parse_stmts.extend(quote! {
                                let #offset_field = #start_cursor;
                                let #len_field = limit;
                            });
                        } else {
                            self.parse_stmts.extend(quote! {
                                let #offset_field = #start_cursor;
                                let #len_field = #name_ident.consumed();
                            });
                        }
                    }

                    self.static_offset = None;
                    self.dynamic_offset = quote! { cursor };

                    if in_conditional {
                        accessors.extend(quote!{
                            pub fn #name_ident(&self) -> Option<Result<(#struct_ident<'a>, usize, usize), binparse::Error>> {
                                let start = self.#offset_field?;
                                let len = self.#len_field?;
                                let slice = &self.data[start..start+len];
                                match #struct_ident::parse(slice) {
                                    Ok(val) => Some(Ok((val, start, len))),
                                    Err(e) => Some(Err(e)),
                                }
                            }
                        });
                    } else {
                        accessors.extend(quote!{
                            pub fn #name_ident(&self) -> Result<(#struct_ident<'a>, usize, usize), binparse::Error> {
                                let start = self.#offset_field;
                                let len = self.#len_field;
                                let slice = &self.data[start..start+len];
                                let val = #struct_ident::parse(slice)?;
                                Ok((val, start, len))
                            }
                        });
                    }
                }
                ast::Type::Union(u) => {
                    self.generate_union(
                        field,
                        u,
                        accessors,
                        extra_types,
                        start_offset_expr.clone(),
                        in_conditional,
                    )?;
                }
            },
            ast::FieldValue::Constraint(lit) => {
                match lit {
                    ast::NumericLiteral::Binary { value, width } => {
                        let width = *width as usize;
                        let val = *value;
                        let val_lit = proc_macro2::Literal::u128_unsuffixed(val);

                        let mut current_bit = 0;
                        if let Some(so) = self.static_offset {
                            current_bit = so.bit;
                        }

                        let total_bits = current_bit + width;
                        let added_bytes = total_bits / 8;
                        let new_bit = total_bits % 8;

                        let bytes_needed = (total_bits + 7) / 8;
                        let bytes_needed_lit = proc_macro2::Literal::usize_unsuffixed(bytes_needed);

                        if current_bit == 0 {
                            self.parse_stmts.extend(quote! {
                                if data.len() < cursor + #bytes_needed_lit {
                                    return Err(binparse::Error::UnexpectedEof);
                                }
                            });
                        }

                        let read_val = if total_bits <= 8 {
                            let shift = 8 - total_bits;
                            let mask = (1u16 << width) - 1;
                            quote! {
                                ((data[cursor] >> #shift) & #mask as u8)
                            }
                        } else {
                            // TODO: Handle bitfields crossing byte boundaries
                            quote! { 0 }
                        };

                        self.parse_stmts.extend(quote! {
                            let val = #read_val;
                            if val != #val_lit as u8 {
                                return Err(binparse::Error::InvalidConst);
                            }
                        });

                        if let Some(so) = self.static_offset {
                            self.static_offset = Some(Len {
                                byte: so.byte + added_bytes,
                                bit: new_bit,
                            });
                        }
                    }
                    ast::NumericLiteral::Hex { value, width } => {
                        let bytes = ((width + 1) / 2) as usize;
                        let val = *value;

                        let bytes_lit = proc_macro2::Literal::usize_unsuffixed(bytes);

                        self.parse_stmts.extend(quote! {
                            if data.len() < cursor + #bytes_lit {
                                return Err(binparse::Error::UnexpectedEof);
                            }
                        });

                        let read_expr = match endian {
                            Endian::Big => match bytes {
                                1 => quote! { data[cursor] as u128 },
                                2 => {
                                    quote! { u16::from_be_bytes(data[cursor..cursor+2].try_into().unwrap()) as u128 }
                                }
                                4 => {
                                    quote! { u32::from_be_bytes(data[cursor..cursor+4].try_into().unwrap()) as u128 }
                                }
                                8 => {
                                    quote! { u64::from_be_bytes(data[cursor..cursor+8].try_into().unwrap()) as u128 }
                                }
                                _ => quote! { 0 },
                            },
                            Endian::Little => match bytes {
                                1 => quote! { data[cursor] as u128 },
                                2 => {
                                    quote! { u16::from_le_bytes(data[cursor..cursor+2].try_into().unwrap()) as u128 }
                                }
                                4 => {
                                    quote! { u32::from_le_bytes(data[cursor..cursor+4].try_into().unwrap()) as u128 }
                                }
                                8 => {
                                    quote! { u64::from_le_bytes(data[cursor..cursor+8].try_into().unwrap()) as u128 }
                                }
                                _ => quote! { 0 },
                            },
                        };

                        let val_lit = proc_macro2::Literal::u128_unsuffixed(val);

                        self.parse_stmts.extend(quote! {
                            let val = #read_expr;
                            if val != #val_lit {
                                return Err(binparse::Error::InvalidConst);
                            }
                            cursor += #bytes_lit;
                        });

                        if let Some(so) = self.static_offset {
                            self.static_offset = Some(
                                so + Len {
                                    byte: bytes,
                                    bit: 0,
                                },
                            );
                        }
                    }
                    ast::NumericLiteral::Decimal(val) => {
                        // Guess size from value
                        let val = *val;
                        let bytes = if val <= 0xFF {
                            1
                        } else if val <= 0xFFFF {
                            2
                        } else if val <= 0xFFFFFFFF {
                            4
                        } else {
                            8
                        };

                        let bytes_lit = proc_macro2::Literal::usize_unsuffixed(bytes);

                        self.parse_stmts.extend(quote! {
                            if data.len() < cursor + #bytes_lit {
                                return Err(binparse::Error::UnexpectedEof);
                            }
                        });

                        let read_expr = match endian {
                            Endian::Big => match bytes {
                                1 => quote! { data[cursor] as u128 },
                                2 => {
                                    quote! { u16::from_be_bytes(data[cursor..cursor+2].try_into().unwrap()) as u128 }
                                }
                                4 => {
                                    quote! { u32::from_be_bytes(data[cursor..cursor+4].try_into().unwrap()) as u128 }
                                }
                                8 => {
                                    quote! { u64::from_be_bytes(data[cursor..cursor+8].try_into().unwrap()) as u128 }
                                }
                                _ => quote! { 0 },
                            },
                            Endian::Little => match bytes {
                                1 => quote! { data[cursor] as u128 },
                                2 => {
                                    quote! { u16::from_le_bytes(data[cursor..cursor+2].try_into().unwrap()) as u128 }
                                }
                                4 => {
                                    quote! { u32::from_le_bytes(data[cursor..cursor+4].try_into().unwrap()) as u128 }
                                }
                                8 => {
                                    quote! { u64::from_le_bytes(data[cursor..cursor+8].try_into().unwrap()) as u128 }
                                }
                                _ => quote! { 0 },
                            },
                        };

                        let val_lit = proc_macro2::Literal::u128_unsuffixed(val);

                        self.parse_stmts.extend(quote! {
                            let val = #read_expr;
                            if val != #val_lit {
                                return Err(binparse::Error::InvalidConst);
                            }
                            cursor += #bytes_lit;
                        });

                        if let Some(so) = self.static_offset {
                            self.static_offset = Some(
                                so + Len {
                                    byte: bytes,
                                    bit: 0,
                                },
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn generate_union(
        &mut self,
        field: &ast::Field<'a>,
        u: &ast::Union<'a>,
        accessors: &mut TokenStream,
        extra_types: &mut TokenStream,
        start_offset_expr: TokenStream,
        in_conditional: bool,
    ) -> Result<(), Error> {
        let field_name_ident = Ident::new_raw(field.name, Span::call_site());
        let parent_name = self.origin.name;
        let enum_name = format!("{}_{}", parent_name, field.name);
        let enum_ident = Ident::new(&enum_name, Span::call_site());

        let mut enum_variants = TokenStream::new();
        let mut parse_arms = TokenStream::new();
        let mut accessor_arms = TokenStream::new();

        let target_expr_parse = if u.target.len() == 1 {
            crate::generate_variable(&u.target[0], &quote! {}, crate::ExprContext::Parse)
        } else {
            let mut vars = Vec::new();
            for target in &u.target {
                vars.push(crate::generate_variable(
                    target,
                    &quote! {},
                    crate::ExprContext::Parse,
                ));
            }
            quote! { (#(#vars),*) }
        };

        let target_expr_accessor = if u.target.len() == 1 {
            crate::generate_variable(&u.target[0], &quote! { self }, crate::ExprContext::Accessor)
        } else {
            let mut vars = Vec::new();
            for target in &u.target {
                vars.push(crate::generate_variable(
                    target,
                    &quote! { self },
                    crate::ExprContext::Accessor,
                ));
            }
            quote! { (#(#vars),*) }
        };

        for variant in &u.variants {
            let pat = if variant.matchers.len() == 1 {
                generate_pattern(&variant.matchers[0])
            } else {
                let pats: Vec<_> = variant.matchers.iter().map(generate_pattern).collect();
                quote! { #(#pats)|* }
            };

            match &variant.body {
                ast::UnionBody::NamedInline(name, items) => {
                    let variant_ident = Ident::new(name, Span::call_site());

                    let variant_struct_def = ast::Struct {
                        name,
                        attributes: vec![],
                        items: items.clone(),
                    };

                    let ctx = StructCtx::new(variant_struct_def, self.done);
                    let generated = ctx.generate()?;
                    extra_types.extend(generated.tokens);

                    enum_variants.extend(quote! {
                        #variant_ident(#variant_ident<'a>),
                    });

                    parse_arms.extend(quote! {
                        #pat => {
                            let val = #variant_ident::parse(&data[cursor..])?;
                            val.consumed()
                        }
                    });

                    accessor_arms.extend(quote! {
                        #pat => {
                             let val = #variant_ident::parse(slice)?;
                             let consumed = val.consumed();
                             Ok((#enum_ident::#variant_ident(val), offset, consumed))
                        }
                    });
                }
                ast::UnionBody::Error(err_name, fields) => {
                    let err_ident = Ident::new(err_name, Span::call_site());

                    let generate_err_fields =
                        |ctx: crate::ExprContext, receiver: &TokenStream| -> TokenStream {
                            let mut tokens = TokenStream::new();
                            for (fname, atom) in fields {
                                let fname_ident = Ident::new(fname, Span::call_site());
                                let val = match atom {
                                    ast::NumericAtom::Literal(lit) => crate::generate_literal(lit),
                                    ast::NumericAtom::Variable(path) => {
                                        crate::generate_variable(path, receiver, ctx)
                                    }
                                };
                                tokens.extend(quote! { #fname_ident: #val as _, });
                            }
                            tokens
                        };

                    let parse_err_fields =
                        generate_err_fields(crate::ExprContext::Parse, &quote! {});
                    let accessor_err_fields =
                        generate_err_fields(crate::ExprContext::Accessor, &quote! {self});

                    parse_arms.extend(quote! {
                        #pat => {
                            return Err(binparse::Error::#err_ident { #parse_err_fields }.into());
                        }
                    });

                    accessor_arms.extend(quote! {
                        #pat => {
                            return Err(binparse::Error::#err_ident { #accessor_err_fields }.into());
                        }
                    });
                }
            }
        }

        extra_types.extend(quote! {
            #[derive(Debug, Clone)]
            pub enum #enum_ident<'a> {
                #enum_variants
            }
        });

        self.static_offset = None;
        self.dynamic_offset = quote! { cursor };

        let offset_field = Ident::new(&format!("_{}_offset", field.name), Span::call_site());

        if in_conditional {
            self.parse_stmts.extend(quote! {
                #offset_field = Some(cursor);
            });
        } else {
            self.extra_fields
                .push((offset_field.clone(), quote! { usize }));
            self.parse_stmts.extend(quote! {
                let #offset_field = cursor;
            });
        }

        self.parse_stmts.extend(quote! {
             let consumed = match #target_expr_parse {
                 #parse_arms
             };
             cursor += consumed;
        });

        let accessor_body = quote! {
             let offset = #start_offset_expr;
             let slice = &self.data[offset..];
             match #target_expr_accessor {
                 #accessor_arms
             }
        };

        if in_conditional {
            accessors.extend(quote! {
                 pub fn #field_name_ident(&self) -> Option<Result<(#enum_ident<'a>, usize, usize), binparse::Error>> {
                     let _ = self.#offset_field?;
                     Some((|| {
                         #accessor_body
                     })())
                 }
             });
        } else {
            accessors.extend(quote! {
                 pub fn #field_name_ident(&self) -> Result<(#enum_ident<'a>, usize, usize), binparse::Error> {
                     #accessor_body
                 }
             });
        }

        Ok(())
    }
}

fn generate_pattern(pattern: &ast::Pattern) -> TokenStream {
    match pattern {
        ast::Pattern::Literal(lit) => crate::generate_literal(lit),
        ast::Pattern::Wildcard => quote! { _ },
        ast::Pattern::Tuple(pats) => {
            let pats: Vec<_> = pats.iter().map(generate_pattern).collect();
            quote! { (#(#pats),*) }
        }
    }
}
