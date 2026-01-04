use std::collections::{HashMap, HashSet};

use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::quote;

pub struct CodeGen<'a> {
    todo: HashMap<&'a str, Todo<'a>>,
    done: HashMap<&'a str, Option<usize>>,
    items: Vec<TokenStream>,
}

pub struct Todo<'a> {
    origin: &'a ast::Struct<'a>,
    dependencies: HashSet<&'a str>,
    dependants: HashSet<&'a str>,
}

impl<'a> Todo<'a> {
    pub fn new(origin: &'a ast::Struct<'a>) -> Self {
        Self {
            origin,
            dependencies: HashSet::new(),
            dependants: HashSet::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("duplicate struct definition '{name}'")]
    DuplicateStruct { name: String },
    #[error("first field '{field}' of struct '{struct_}' is a dynamic type")]
    FirstFieldDynamic { struct_: String, field: String },
    #[error("field '{field}' of struct '{struct_}' is unaligned")]
    UnalignedType { struct_: String, field: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Len {
    byte: usize,
    bit: usize,
}

impl std::ops::Add for Len {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut byte = self.byte + other.byte;
        let mut bit = self.bit + other.bit;
        if bit >= 8 {
            byte += 1;
            bit -= 8;
        }
        Self { byte, bit }
    }
}

impl<'a> CodeGen<'a> {
    pub fn generate(ast: &'a [ast::Definition<'a>]) -> Result<String, Error> {
        let mut structs = HashMap::new();
        let mut reverse_deps = HashMap::<_, HashSet<_>>::new();

        let mut roots = Vec::new();
        #[expect(unused)]
        let mut error_enum = None;

        for def in ast {
            let s = match def {
                ast::Definition::Struct(s) => s,
                #[expect(unused_assignments)]
                ast::Definition::Error(variants) => {
                    error_enum = Some(variants);
                    continue;
                }
            };

            let mut new_s = Todo::new(s);
            find_dependencies(&s.items, &mut new_s.dependencies);

            if new_s.dependencies.is_empty() {
                roots.push(s.name);
            } else {
                for dependency in &new_s.dependencies {
                    reverse_deps.entry(*dependency).or_default().insert(s.name);
                }
            }

            if structs.insert(s.name, new_s).is_some() {
                return Err(Error::DuplicateStruct {
                    name: s.name.to_owned(),
                });
            }
        }

        for (struct_, actual) in reverse_deps {
            let dependants = &mut structs.get_mut(struct_).expect("dependant not found when they should have been added when building dependencies").dependants;
            *dependants = actual;
        }

        let mut me = Self {
            todo: structs,
            done: HashMap::new(),
            items: Vec::new(),
        };
        let mut next = Vec::new();
        while !roots.is_empty() {
            for root in roots.drain(..) {
                let todo = me.todo.remove(root).expect("root not found in todo");
                me.generate_struct(todo.origin)?;
            }

            std::mem::swap(&mut next, &mut roots);
        }

        me.print()
    }

    fn generate_struct(&mut self, struct_: &'a ast::Struct<'a>) -> Result<(), Error> {
        let name_ident = quote::format_ident!("{}", struct_.name);

        let mut impl_items = Vec::<TokenStream>::new();

        let mut total_len_opt = Some(Len::default());

        for (index, field) in struct_.items.iter().enumerate() {
            match field {
                ast::StructItem::Field(ast::Field {
                    name,
                    attributes: _,
                    value: ast::FieldValue::Type(ty),
                }) => {
                    let (len_opt, tokens) =
                        self.generate_field(name, ty, struct_, total_len_opt)?;
                    impl_items.push(tokens);

                    match (len_opt, total_len_opt) {
                        (Some(len), Some(current)) => total_len_opt = Some(current + len),
                        (None, Some(_)) => {
                            if index == 0 {
                                return Err(Error::FirstFieldDynamic {
                                    struct_: struct_.name.to_string(),
                                    field: name.to_string(),
                                });
                            }
                            total_len_opt = None;
                        }
                        _ => total_len_opt = None,
                    }
                }
                _ => todo!("handle conditional fields in first field"),
            }
        }

        let tokens = quote! {
            pub struct #name_ident<'a> {
                pub read_buffer: &'a [u8],
            }

            impl #name_ident<'_> {
                #(#impl_items)*
            }
        };

        self.items.push(tokens);
        self.done.insert(struct_.name, None);
        Ok(())
    }

    #[expect(dead_code, unused_variables)]
    fn generate_conditional(&mut self, conditional: &'a ast::Conditional<'a>) -> Result<(), Error> {
        todo!()
    }

    fn generate_field(
        &self,
        name: &str,
        ty: &'a ast::Type<'a>,
        struct_: &'a ast::Struct<'a>,
        offset: Option<Len>,
    ) -> Result<(Option<Len>, TokenStream), Error> {
        match ty {
            ast::Type::Primitive(primitive) => {
                let (len, rust_ty, needs_alignment) = match_primitive(primitive);

                if let Some(offset) = offset {
                    let bits = offset.bit;
                    let bytes = offset.byte;
                    let len_bytes = len.byte;
                    let len_bits = len.bit;

                    let end_offset = offset + len;
                    let end_bytes = end_offset.byte;
                    let end_bits = end_offset.bit;

                    let end_offset_fn_name = quote::format_ident!("{}_end_offset", name);
                    let end_offset_fn = quote! {
                        pub fn #end_offset_fn_name(&self) -> Len {
                            Len { byte: #end_bytes, bit: #end_bits }
                        }
                    };

                    let getter_fn_name = quote::format_ident!("{}", name);

                    let getter_body = if needs_alignment {
                        if bits == 0 && len_bits == 0 {
                            quote! {
                                let slice = &self.read_buffer[#bytes..#bytes + #len_bytes];
                                let array = slice.try_into().expect("slice has correct length");
                                #rust_ty::from_be_bytes(array)
                            }
                        } else {
                            return Err(Error::UnalignedType {
                                struct_: struct_.name.to_string(),
                                field: name.to_string(),
                            });
                        }
                    } else {
                        // len_bytes == 0, we don't allow longer BitFields in parser
                        if len_bits + bits > 8 {
                            quote! {
                                let slice = &self.read_buffer[#bytes..#bytes + #len_bytes];
                            }
                        } else {
                            let val = (1 << len_bits) - 1;
                            let mask_literal = syn::LitInt::new(
                                &format!("0b{:b}", val),
                                proc_macro2::Span::call_site(),
                            );
                            quote! {
                                let byte = self.read_buffer[#bytes];
                                (byte >> #bits) & #mask_literal
                            }
                        }
                    };

                    let getter_fn = quote! {
                        pub fn #getter_fn_name(&self) -> #rust_ty {
                            #getter_body
                        }
                    };

                    Ok((
                        Some(len),
                        quote! {
                            #end_offset_fn
                            #getter_fn
                        },
                    ))
                } else {
                    Ok((Some(len), todo!()))
                }
            }
            _ => todo!("support non-primitive types"),
        }
    }

    fn print(self) -> Result<String, Error> {
        let combined: TokenStream = self.items.into_iter().collect();
        let file: syn::File = syn::parse2(combined).expect("failed to parse generated tokens");
        Ok(prettyplease::unparse(&file))
    }
}

fn find_dependencies<'a>(
    struct_items: &[ast::StructItem<'a>],
    dependencies: &mut HashSet<&'a str>,
) {
    for item in struct_items {
        match item {
            ast::StructItem::Field(ast::Field {
                value: ast::FieldValue::Type(ty),
                ..
            }) => find_type_dependencies(ty, dependencies),
            ast::StructItem::Conditional(conditional) => {
                find_conditional_dependencies(conditional, dependencies)
            }
            _ => {}
        }
    }
}

fn find_type_dependencies<'a>(ty: &ast::Type<'a>, dependencies: &mut HashSet<&'a str>) {
    match ty {
        ast::Type::StructRef(path) => {
            dependencies.insert(path.last().unwrap());
        }
        ast::Type::Array(array_ty) => {
            find_type_dependencies(&array_ty.elem_ty, dependencies);
        }
        ast::Type::Concat(fields) => {
            for field in fields {
                if let ast::FieldValue::Type(ty) = &field.value {
                    find_type_dependencies(ty, dependencies);
                }
            }
        }
        ast::Type::Union(union) => {
            for variant in &union.variants {
                if let ast::UnionBody::NamedInline(_, items) = &variant.body {
                    find_dependencies(items, dependencies);
                }
            }
        }
        _ => {}
    }
}

fn find_conditional_dependencies<'a>(
    conditional: &ast::Conditional<'a>,
    dependencies: &mut HashSet<&'a str>,
) {
    find_dependencies(&conditional.then_branch, dependencies);
    if let Some(else_branch) = &conditional.else_branch {
        find_dependencies(else_branch, dependencies);
    }
}

fn match_primitive(primitive: &ast::Primitive) -> (Len, TokenStream, bool) {
    match primitive {
        ast::Primitive::U8 => (Len { byte: 1, bit: 0 }, quote! { u8 }, true),
        ast::Primitive::U16 => (Len { byte: 2, bit: 0 }, quote! { u16 }, true),
        ast::Primitive::U32 => (Len { byte: 4, bit: 0 }, quote! { u32 }, true),
        ast::Primitive::U64 => (Len { byte: 8, bit: 0 }, quote! { u64 }, true),
        ast::Primitive::U128 => (Len { byte: 16, bit: 0 }, quote! { u128 }, true),
        ast::Primitive::BitField(width) => (
            Len {
                byte: 0,
                bit: *width as usize % 8,
            },
            quote! { u8 },
            false,
        ),
    }
}
