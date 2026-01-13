#[cfg(test)]
mod test_utils {
    use binparse_dsl as ast;
    use proc_macro2::TokenStream;

    pub fn tokens_to_string(tokens: TokenStream) -> String {
        let file: syn::File =
            syn::parse2(tokens).expect("failed to parse generated tokens as syn::File");
        prettyplease::unparse(&file)
    }

    pub fn assert_tokens_eq(actual: TokenStream, expected: &str) {
        let actual_str = tokens_to_string(actual);
        let expected_str = {
            let file: syn::File =
                syn::parse_str(expected).expect("failed to parse expected string as syn::File");
            prettyplease::unparse(&file)
        };

        if actual_str != expected_str {
            panic!(
                "Generated code does not match expected.\n\n--- Expected ---\n{}\n\n--- Actual ---\n{}\n",
                expected_str, actual_str
            );
        }
    }

    pub fn parse_single_struct(dsl: &str) -> ast::Struct<'_> {
        let defs = binparse_dsl_parse::parse_str(dsl).unwrap();
        assert_eq!(defs.len(), 1);
        match &defs[0] {
            ast::Definition::Struct(s) => s.clone(),
            _ => panic!("expected struct"),
        }
    }

    pub fn extract_field_type<'a>(s: &'a ast::Struct<'a>) -> &'a ast::Type<'a> {
        assert_eq!(s.items.len(), 1);
        match &s.items[0] {
            ast::StructItem::Field(f) => match &f.value {
                ast::FieldValue::Type(ty) => ty,
                _ => panic!("expected type"),
            },
            _ => panic!("expected field"),
        }
    }
}

#[cfg(test)]
mod primitive_tests {
    use binparse::Len;
    use binparse_dsl as ast;
    use quote::format_ident;

    use crate::type_::primitive::PrimitiveCtx;

    use super::test_utils::{assert_tokens_eq, extract_field_type, parse_single_struct};

    fn extract_primitive<'a>(ty: &'a ast::Type<'a>) -> &'a ast::Primitive {
        match ty {
            ast::Type::Primitive(p) => p,
            _ => panic!("expected primitive"),
        }
    }

    #[test]
    fn test_u8_at_offset_0() {
        let dsl = "struct Foo { my_field: u8 }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let primitive = extract_primitive(ty);

        let field_name = format_ident!("my_field");
        let ctx = PrimitiveCtx {
            primitive,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 1, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn my_field(&self) -> u8 {
                u8::from_ne_bytes(self.data[0usize..1usize].try_into().unwrap())
            }
            "#,
        );
    }

    #[test]
    fn test_u8_at_offset_5() {
        let dsl = "struct Foo { byte_field: u8 }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let primitive = extract_primitive(ty);

        let field_name = format_ident!("byte_field");
        let ctx = PrimitiveCtx {
            primitive,
            field_name: &field_name,
            start_offset: Some(Len { byte: 5, bit: 0 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 1, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn byte_field(&self) -> u8 {
                u8::from_ne_bytes(self.data[5usize..6usize].try_into().unwrap())
            }
            "#,
        );
    }

    #[test]
    fn test_u16_at_offset_0() {
        let dsl = "struct Foo { short_field: u16 }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let primitive = extract_primitive(ty);

        let field_name = format_ident!("short_field");
        let ctx = PrimitiveCtx {
            primitive,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 2, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn short_field(&self) -> u16 {
                u16::from_ne_bytes(self.data[0usize..2usize].try_into().unwrap())
            }
            "#,
        );
    }

    #[test]
    fn test_u32_at_offset_4() {
        let dsl = "struct Foo { int_field: u32 }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let primitive = extract_primitive(ty);

        let field_name = format_ident!("int_field");
        let ctx = PrimitiveCtx {
            primitive,
            field_name: &field_name,
            start_offset: Some(Len { byte: 4, bit: 0 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 4, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn int_field(&self) -> u32 {
                u32::from_ne_bytes(self.data[4usize..8usize].try_into().unwrap())
            }
            "#,
        );
    }

    #[test]
    fn test_u64_at_offset_0() {
        let dsl = "struct Foo { long_field: u64 }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let primitive = extract_primitive(ty);

        let field_name = format_ident!("long_field");
        let ctx = PrimitiveCtx {
            primitive,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 8, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn long_field(&self) -> u64 {
                u64::from_ne_bytes(self.data[0usize..8usize].try_into().unwrap())
            }
            "#,
        );
    }

    #[test]
    fn test_u128_at_offset_8() {
        let dsl = "struct Foo { huge_field: u128 }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let primitive = extract_primitive(ty);

        let field_name = format_ident!("huge_field");
        let ctx = PrimitiveCtx {
            primitive,
            field_name: &field_name,
            start_offset: Some(Len { byte: 8, bit: 0 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 16, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn huge_field(&self) -> u128 {
                u128::from_ne_bytes(self.data[8usize..24usize].try_into().unwrap())
            }
            "#,
        );
    }

    #[test]
    fn test_primitive_with_unaligned_offset_fails() {
        let dsl = "struct Foo { bad_field: u16 }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let primitive = extract_primitive(ty);

        let field_name = format_ident!("bad_field");
        let ctx = PrimitiveCtx {
            primitive,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 3 }),
        };

        let result = ctx.generate();
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod bitfield_tests {
    use binparse::Len;
    use binparse_dsl as ast;
    use quote::format_ident;

    use crate::type_::bitfield::BitFieldCtx;

    use super::test_utils::{assert_tokens_eq, extract_field_type, parse_single_struct};

    fn extract_bitfield_width(ty: &ast::Type<'_>) -> usize {
        match ty {
            ast::Type::BitField(w) => *w as usize,
            _ => panic!("expected bitfield"),
        }
    }

    #[test]
    fn test_4bit_at_bit_0() {
        let dsl = "struct Foo { nibble: b<4> }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let width = extract_bitfield_width(ty);

        let field_name = format_ident!("nibble");
        let ctx = BitFieldCtx {
            width,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 4 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn nibble(&self) -> u8 {
                (self.data[0usize] >> 0usize) & 15u8
            }
            "#,
        );
    }

    #[test]
    fn test_4bit_at_bit_4() {
        let dsl = "struct Foo { high_nibble: b<4> }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let width = extract_bitfield_width(ty);

        let field_name = format_ident!("high_nibble");
        let ctx = BitFieldCtx {
            width,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 4 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 4 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn high_nibble(&self) -> u8 {
                (self.data[0usize] >> 4usize) & 15u8
            }
            "#,
        );
    }

    #[test]
    fn test_3bit_at_bit_0() {
        let dsl = "struct Foo { flags: b<3> }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let width = extract_bitfield_width(ty);

        let field_name = format_ident!("flags");
        let ctx = BitFieldCtx {
            width,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 3 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn flags(&self) -> u8 {
                (self.data[0usize] >> 0usize) & 7u8
            }
            "#,
        );
    }

    #[test]
    fn test_6bit_at_bit_0() {
        let dsl = "struct Foo { dscp: b<6> }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let width = extract_bitfield_width(ty);

        let field_name = format_ident!("dscp");
        let ctx = BitFieldCtx {
            width,
            field_name: &field_name,
            start_offset: Some(Len { byte: 1, bit: 0 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 6 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn dscp(&self) -> u8 {
                (self.data[1usize] >> 0usize) & 63u8
            }
            "#,
        );
    }

    #[test]
    fn test_2bit_at_bit_6() {
        let dsl = "struct Foo { ecn: b<2> }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let width = extract_bitfield_width(ty);

        let field_name = format_ident!("ecn");
        let ctx = BitFieldCtx {
            width,
            field_name: &field_name,
            start_offset: Some(Len { byte: 1, bit: 6 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 2 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn ecn(&self) -> u8 {
                (self.data[1usize] >> 6usize) & 3u8
            }
            "#,
        );
    }

    #[test]
    fn test_5bit_crossing_byte_boundary() {
        let dsl = "struct Foo { frag_hi: b<5> }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let width = extract_bitfield_width(ty);

        let field_name = format_ident!("frag_hi");
        let ctx = BitFieldCtx {
            width,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 5 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 5 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn frag_hi(&self) -> u8 {
                let first_part = (self.data[0usize] >> 5usize) & 7u8;
                let second_part = self.data[1usize] & 3u8;
                first_part | (second_part << 3usize)
            }
            "#,
        );
    }

    #[test]
    fn test_7bit_crossing_byte_boundary() {
        let dsl = "struct Foo { big_field: b<7> }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let width = extract_bitfield_width(ty);

        let field_name = format_ident!("big_field");
        let ctx = BitFieldCtx {
            width,
            field_name: &field_name,
            start_offset: Some(Len { byte: 2, bit: 3 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 7 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn big_field(&self) -> u8 {
                let first_part = (self.data[2usize] >> 3usize) & 31u8;
                let second_part = self.data[3usize] & 3u8;
                first_part | (second_part << 5usize)
            }
            "#,
        );
    }

    #[test]
    fn test_1bit_field() {
        let dsl = "struct Foo { single_bit: b<1> }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let width = extract_bitfield_width(ty);

        let field_name = format_ident!("single_bit");
        let ctx = BitFieldCtx {
            width,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 7 }),
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 1 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn single_bit(&self) -> u8 {
                (self.data[0usize] >> 7usize) & 1u8
            }
            "#,
        );
    }
}

#[cfg(test)]
mod struct_ref_tests {
    use std::collections::HashMap;

    use binparse::Len;
    use binparse_dsl as ast;
    use quote::{format_ident, quote};

    use crate::struct_::GeneratedStruct;
    use crate::type_::struct_ref::StructRefCtx;

    use super::test_utils::{assert_tokens_eq, extract_field_type, parse_single_struct};

    fn extract_struct_ref<'a>(ty: &'a ast::Type<'a>) -> &'a str {
        match ty {
            ast::Type::StructRef(name) => name,
            _ => panic!("expected struct ref"),
        }
    }

    #[test]
    fn test_struct_ref_at_offset_0() {
        let dsl = "struct Foo { inner: Inner }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let struct_name = extract_struct_ref(ty);

        let field_name = format_ident!("inner");

        let mut done = HashMap::new();
        done.insert(
            "Inner",
            GeneratedStruct {
                len: Some(Len { byte: 4, bit: 0 }),
                tokens: quote! {},
            },
        );

        let ctx = StructRefCtx {
            struct_name,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
            done: &done,
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 4, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn inner(&self) -> Inner<'_> {
                Inner::parse(&self.data[0usize..]).unwrap().0
            }
            "#,
        );
    }

    #[test]
    fn test_struct_ref_at_offset_8() {
        let dsl = "struct Foo { nested: NestedStruct }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let struct_name = extract_struct_ref(ty);

        let field_name = format_ident!("nested");

        let mut done = HashMap::new();
        done.insert(
            "NestedStruct",
            GeneratedStruct {
                len: Some(Len { byte: 12, bit: 0 }),
                tokens: quote! {},
            },
        );

        let ctx = StructRefCtx {
            struct_name,
            field_name: &field_name,
            start_offset: Some(Len { byte: 8, bit: 0 }),
            done: &done,
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 12, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn nested(&self) -> NestedStruct<'_> {
                NestedStruct::parse(&self.data[8usize..]).unwrap().0
            }
            "#,
        );
    }

    #[test]
    fn test_struct_ref_unknown_type() {
        let dsl = "struct Foo { unknown: UnknownType }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let struct_name = extract_struct_ref(ty);

        let field_name = format_ident!("unknown");
        let done = HashMap::new();

        let ctx = StructRefCtx {
            struct_name,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
            done: &done,
        };

        let result = ctx.generate();
        assert!(result.is_err());
    }

    #[test]
    fn test_struct_ref_unaligned_offset_fails() {
        let dsl = "struct Foo { bad: Inner }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let struct_name = extract_struct_ref(ty);

        let field_name = format_ident!("bad");

        let mut done = HashMap::new();
        done.insert(
            "Inner",
            GeneratedStruct {
                len: Some(Len { byte: 4, bit: 0 }),
                tokens: quote! {},
            },
        );

        let ctx = StructRefCtx {
            struct_name,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 2 }),
            done: &done,
        };

        let result = ctx.generate();
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod concat_tests {
    use std::collections::HashMap;

    use binparse::Len;
    use binparse_dsl as ast;
    use quote::format_ident;

    use crate::struct_::GeneratedStruct;
    use crate::type_::concat::ConcatCtx;

    use super::test_utils::{assert_tokens_eq, extract_field_type, parse_single_struct};

    fn extract_concat_items<'a>(ty: &'a ast::Type<'a>) -> &'a [ast::ConcatItem<'a>] {
        match ty {
            ast::Type::Concat(items) => items,
            _ => panic!("expected concat"),
        }
    }

    #[test]
    fn test_concat_two_bitfields() {
        let dsl = "struct Foo { fragment: concat(b<5>, b<3>) }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let items = extract_concat_items(ty);

        let field_name = format_ident!("fragment");
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = ConcatCtx {
            items,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
            done: &done,
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 1, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn fragment_0(&self) -> u8 {
                (self.data[0usize] >> 0usize) & 31u8
            }
            #[allow(clippy::identity_op)]
            pub fn fragment_1(&self) -> u8 {
                (self.data[0usize] >> 5usize) & 7u8
            }
            pub fn fragment(&self) -> (u8, u8) {
                (self.fragment_0(), self.fragment_1())
            }
            "#,
        );
    }

    #[test]
    fn test_concat_bitfield_and_primitive() {
        let dsl = "struct Foo { mixed: concat(b<5>, u8) }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let items = extract_concat_items(ty);

        let field_name = format_ident!("mixed");
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = ConcatCtx {
            items,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 3 }),
            done: &done,
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 1, bit: 5 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn mixed_0(&self) -> u8 {
                (self.data[0usize] >> 3usize) & 31u8
            }
            pub fn mixed_1(&self) -> u8 {
                u8::from_ne_bytes(self.data[1usize..2usize].try_into().unwrap())
            }
            pub fn mixed(&self) -> (u8, u8) {
                (self.mixed_0(), self.mixed_1())
            }
            "#,
        );
    }

    #[test]
    fn test_concat_three_primitives() {
        let dsl = "struct Foo { triple: concat(u8, u16, u8) }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let items = extract_concat_items(ty);

        let field_name = format_ident!("triple");
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = ConcatCtx {
            items,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
            done: &done,
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 4, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn triple_0(&self) -> u8 {
                u8::from_ne_bytes(self.data[0usize..1usize].try_into().unwrap())
            }
            pub fn triple_1(&self) -> u16 {
                u16::from_ne_bytes(self.data[1usize..3usize].try_into().unwrap())
            }
            pub fn triple_2(&self) -> u8 {
                u8::from_ne_bytes(self.data[3usize..4usize].try_into().unwrap())
            }
            pub fn triple(&self) -> (u8, u16, u8) {
                (self.triple_0(), self.triple_1(), self.triple_2())
            }
            "#,
        );
    }
}

#[cfg(test)]
mod array_tests {
    use std::collections::HashMap;

    use binparse::Len;
    use binparse_dsl as ast;
    use quote::{format_ident, quote};

    use crate::struct_::GeneratedStruct;
    use crate::type_::array::ArrayCtx;

    use super::test_utils::{assert_tokens_eq, extract_field_type, parse_single_struct};

    fn extract_array_type<'a>(ty: &'a ast::Type<'a>) -> &'a ast::ArrayType<'a> {
        match ty {
            ast::Type::Array(arr) => arr,
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_array_u8_fixed_size() {
        let dsl = "struct Foo { bytes: [u8; 4] }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let array_type = extract_array_type(ty);

        let field_name = format_ident!("bytes");
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = ArrayCtx {
            array_type,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
            done: &done,
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 4, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn bytes(&self) -> impl Iterator<Item = u8> + '_ {
                (0..4usize).map(move |i| {
                    let offset = 0usize + i * 1usize;
                    u8::from_ne_bytes(self.data[offset..offset + 1usize].try_into().unwrap())
                })
            }
            "#,
        );
    }

    #[test]
    fn test_array_u16_fixed_size() {
        let dsl = "struct Foo { shorts: [u16; 8] }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let array_type = extract_array_type(ty);

        let field_name = format_ident!("shorts");
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = ArrayCtx {
            array_type,
            field_name: &field_name,
            start_offset: Some(Len { byte: 4, bit: 0 }),
            done: &done,
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 16, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn shorts(&self) -> impl Iterator<Item = u16> + '_ {
                (0..8usize).map(move |i| {
                    let offset = 4usize + i * 2usize;
                    u16::from_ne_bytes(self.data[offset..offset + 2usize].try_into().unwrap())
                })
            }
            "#,
        );
    }

    #[test]
    fn test_array_u32_fixed_size() {
        let dsl = "struct Foo { ints: [u32; 3] }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let array_type = extract_array_type(ty);

        let field_name = format_ident!("ints");
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = ArrayCtx {
            array_type,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
            done: &done,
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 12, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn ints(&self) -> impl Iterator<Item = u32> + '_ {
                (0..3usize).map(move |i| {
                    let offset = 0usize + i * 4usize;
                    u32::from_ne_bytes(self.data[offset..offset + 4usize].try_into().unwrap())
                })
            }
            "#,
        );
    }

    #[test]
    fn test_array_struct_ref_fixed_size() {
        let dsl = "struct Foo { items: [Item; 5] }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let array_type = extract_array_type(ty);

        let field_name = format_ident!("items");

        let mut done = HashMap::new();
        done.insert(
            "Item",
            GeneratedStruct {
                len: Some(Len { byte: 8, bit: 0 }),
                tokens: quote! {},
            },
        );

        let ctx = ArrayCtx {
            array_type,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 0 }),
            done: &done,
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 40, bit: 0 }));
        let getter_str = result.field_getter.to_string();
        assert!(getter_str.contains("items"));
        assert!(getter_str.contains("Iterator"));
        assert!(getter_str.contains("Item"));
    }

    #[test]
    fn test_array_bitfield_fixed_size() {
        let dsl = "struct Foo { nibbles: [b<4>; 3] }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let array_type = extract_array_type(ty);

        let field_name = format_ident!("nibbles");
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = ArrayCtx {
            array_type,
            field_name: &field_name,
            start_offset: Some(Len { byte: 2, bit: 0 }),
            done: &done,
        };

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 3, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn nibbles(&self) -> impl Iterator<Item = u8> + '_ {
                (0..3usize).map(move |i| {
                    let offset = 2usize + i * 1usize;
                    self.data[offset] & 15u8
                })
            }
            "#,
        );
    }

    #[test]
    fn test_array_unaligned_offset_fails() {
        let dsl = "struct Foo { bad: [u8; 4] }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let array_type = extract_array_type(ty);

        let field_name = format_ident!("bad");
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = ArrayCtx {
            array_type,
            field_name: &field_name,
            start_offset: Some(Len { byte: 0, bit: 3 }),
            done: &done,
        };

        let result = ctx.generate();
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod type_ctx_tests {
    use std::collections::HashMap;

    use binparse::Len;
    use binparse_dsl as ast;
    use quote::{format_ident, quote};

    use crate::struct_::GeneratedStruct;
    use crate::type_::TypeCtx;

    use super::test_utils::{assert_tokens_eq, extract_field_type, parse_single_struct};

    fn extract_array_elem_type<'a>(ty: &'a ast::Type<'a>) -> &'a ast::ArrayElemType<'a> {
        match ty {
            ast::Type::Array(arr) => &arr.elem_ty,
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_type_ctx_dispatches_to_primitive() {
        let dsl = "struct Foo { value: u32 }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);

        let done = HashMap::<&str, GeneratedStruct>::new();
        let ctx = TypeCtx { done: &done };
        let field_name = format_ident!("value");

        let result = ctx
            .generate(ty, &field_name, Some(Len { byte: 0, bit: 0 }))
            .unwrap();

        assert_eq!(result.len, Some(Len { byte: 4, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn value(&self) -> u32 {
                u32::from_ne_bytes(self.data[0usize..4usize].try_into().unwrap())
            }
            "#,
        );
    }

    #[test]
    fn test_type_ctx_dispatches_to_bitfield() {
        let dsl = "struct Foo { flags: b<3> }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);

        let done = HashMap::<&str, GeneratedStruct>::new();
        let ctx = TypeCtx { done: &done };
        let field_name = format_ident!("flags");

        let result = ctx
            .generate(ty, &field_name, Some(Len { byte: 0, bit: 0 }))
            .unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 3 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn flags(&self) -> u8 {
                (self.data[0usize] >> 0usize) & 7u8
            }
            "#,
        );
    }

    #[test]
    fn test_type_ctx_dispatches_to_struct_ref() {
        let dsl = "struct Foo { header: Header }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);

        let mut done = HashMap::new();
        done.insert(
            "Header",
            GeneratedStruct {
                len: Some(Len { byte: 20, bit: 0 }),
                tokens: quote! {},
            },
        );
        let ctx = TypeCtx { done: &done };
        let field_name = format_ident!("header");

        let result = ctx
            .generate(ty, &field_name, Some(Len { byte: 0, bit: 0 }))
            .unwrap();

        assert_eq!(result.len, Some(Len { byte: 20, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn header(&self) -> Header<'_> {
                Header::parse(&self.data[0usize..]).unwrap().0
            }
            "#,
        );
    }

    #[test]
    fn test_type_ctx_dispatches_to_array() {
        let dsl = "struct Foo { data: [u8; 10] }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);

        let done = HashMap::<&str, GeneratedStruct>::new();
        let ctx = TypeCtx { done: &done };
        let field_name = format_ident!("data");

        let result = ctx
            .generate(ty, &field_name, Some(Len { byte: 0, bit: 0 }))
            .unwrap();

        assert_eq!(result.len, Some(Len { byte: 10, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn data(&self) -> impl Iterator<Item = u8> + '_ {
                (0..10usize).map(move |i| {
                    let offset = 0usize + i * 1usize;
                    u8::from_ne_bytes(self.data[offset..offset + 1usize].try_into().unwrap())
                })
            }
            "#,
        );
    }

    #[test]
    fn test_type_ctx_dispatches_to_concat() {
        let dsl = "struct Foo { combo: concat(b<4>, b<4>) }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);

        let done = HashMap::<&str, GeneratedStruct>::new();
        let ctx = TypeCtx { done: &done };
        let field_name = format_ident!("combo");

        let result = ctx
            .generate(ty, &field_name, Some(Len { byte: 0, bit: 0 }))
            .unwrap();

        assert_eq!(result.len, Some(Len { byte: 1, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn combo_0(&self) -> u8 {
                (self.data[0usize] >> 0usize) & 15u8
            }
            #[allow(clippy::identity_op)]
            pub fn combo_1(&self) -> u8 {
                (self.data[0usize] >> 4usize) & 15u8
            }
            pub fn combo(&self) -> (u8, u8) {
                (self.combo_0(), self.combo_1())
            }
            "#,
        );
    }

    #[test]
    fn test_generate_array_elem_primitive() {
        let dsl = "struct Foo { elem: [u16; 1] }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let elem_ty = extract_array_elem_type(ty);

        let done = HashMap::<&str, GeneratedStruct>::new();
        let ctx = TypeCtx { done: &done };
        let field_name = format_ident!("elem");

        let result = ctx
            .generate_array_elem(elem_ty, &field_name, Some(Len { byte: 0, bit: 0 }))
            .unwrap();

        assert_eq!(result.len, Some(Len { byte: 2, bit: 0 }));
    }

    #[test]
    fn test_generate_array_elem_bitfield() {
        let dsl = "struct Foo { elem: [b<4>; 1] }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let elem_ty = extract_array_elem_type(ty);

        let done = HashMap::<&str, GeneratedStruct>::new();
        let ctx = TypeCtx { done: &done };
        let field_name = format_ident!("elem");

        let result = ctx
            .generate_array_elem(elem_ty, &field_name, Some(Len { byte: 0, bit: 0 }))
            .unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 4 }));
    }

    #[test]
    fn test_generate_array_elem_struct_ref() {
        let dsl = "struct Foo { elem: [Inner; 1] }";
        let s = parse_single_struct(dsl);
        let ty = extract_field_type(&s);
        let elem_ty = extract_array_elem_type(ty);

        let mut done = HashMap::new();
        done.insert(
            "Inner",
            GeneratedStruct {
                len: Some(Len { byte: 4, bit: 0 }),
                tokens: quote! {},
            },
        );
        let ctx = TypeCtx { done: &done };
        let field_name = format_ident!("elem");

        let result = ctx
            .generate_array_elem(elem_ty, &field_name, Some(Len { byte: 0, bit: 0 }))
            .unwrap();

        assert_eq!(result.len, Some(Len { byte: 4, bit: 0 }));
    }
}

#[cfg(test)]
mod field_ctx_tests {
    use std::collections::HashMap;

    use binparse::Len;
    use binparse_dsl as ast;
    use quote::{format_ident, quote};

    use crate::field::FieldCtx;
    use crate::struct_::{DoneField, GeneratedStruct};

    use super::test_utils::{assert_tokens_eq, parse_single_struct};

    fn extract_field<'a>(s: &'a ast::Struct<'a>) -> &'a ast::Field<'a> {
        assert_eq!(s.items.len(), 1);
        match &s.items[0] {
            ast::StructItem::Field(f) => f,
            _ => panic!("expected field"),
        }
    }

    fn extract_fields<'a>(s: &'a ast::Struct<'a>) -> Vec<&'a ast::Field<'a>> {
        s.items
            .iter()
            .map(|item| match item {
                ast::StructItem::Field(f) => f,
                _ => panic!("expected field"),
            })
            .collect()
    }

    #[test]
    fn test_field_with_primitive_type() {
        let dsl = "struct Foo { count: u32 }";
        let s = parse_single_struct(dsl);
        let field = extract_field(&s);

        let done = HashMap::<&str, GeneratedStruct>::new();
        let done_fields = vec![];

        let ctx = FieldCtx::new(field, Some(Len { byte: 0, bit: 0 }), &done_fields, &done);

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 4, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn count(&self) -> u32 {
                u32::from_ne_bytes(self.data[0usize..4usize].try_into().unwrap())
            }
            "#,
        );
        assert_tokens_eq(
            result.offset_getter,
            r#"
            pub fn count_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 4usize,
                    bit: 0usize,
                }
            }
            "#,
        );
    }

    #[test]
    fn test_field_with_bitfield_type() {
        let dsl = "struct Foo { version: b<4> }";
        let s = parse_single_struct(dsl);
        let field = extract_field(&s);

        let done = HashMap::<&str, GeneratedStruct>::new();
        let done_fields = vec![];

        let ctx = FieldCtx::new(field, Some(Len { byte: 0, bit: 0 }), &done_fields, &done);

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 4 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            #[allow(clippy::identity_op)]
            pub fn version(&self) -> u8 {
                (self.data[0usize] >> 0usize) & 15u8
            }
            "#,
        );
        assert_tokens_eq(
            result.offset_getter,
            r#"
            pub fn version_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 0usize,
                    bit: 4usize,
                }
            }
            "#,
        );
    }

    #[test]
    fn test_field_at_nonzero_offset() {
        let dsl = "struct Foo { length: u16 }";
        let s = parse_single_struct(dsl);
        let field = extract_field(&s);

        let done = HashMap::<&str, GeneratedStruct>::new();
        let done_fields = vec![];

        let ctx = FieldCtx::new(field, Some(Len { byte: 4, bit: 0 }), &done_fields, &done);

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 2, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn length(&self) -> u16 {
                u16::from_ne_bytes(self.data[4usize..6usize].try_into().unwrap())
            }
            "#,
        );
        assert_tokens_eq(
            result.offset_getter,
            r#"
            pub fn length_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 6usize,
                    bit: 0usize,
                }
            }
            "#,
        );
    }

    #[test]
    #[ignore = "None offset not yet implemented for bitfields"]
    fn test_field_uses_prev_offset_when_no_start_offset() {
        let dsl = "struct Foo { prev: u8, current: b<4> }";
        let s = parse_single_struct(dsl);
        let fields = extract_fields(&s);
        let prev_field = fields[0];
        let field = fields[1];

        let done_fields = vec![DoneField {
            origin: prev_field,
            len: Some(Len { byte: 1, bit: 0 }),
            offset_getter_fn_name: format_ident!("prev_end_offset"),
        }];

        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = FieldCtx::new(field, None, &done_fields, &done);

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 4 }));
        assert_tokens_eq(
            result.offset_getter,
            r#"
            pub fn current_end_offset(&self) -> binparse::Len {
                let prev = self.prev_end_offset();
                binparse::Len {
                    byte: prev.byte + 0usize,
                    bit: prev.bit + 4usize,
                }
            }
            "#,
        );
    }

    #[test]
    fn test_field_with_struct_ref() {
        let dsl = "struct Foo { header: Header }";
        let s = parse_single_struct(dsl);
        let field = extract_field(&s);

        let mut done = HashMap::new();
        done.insert(
            "Header",
            GeneratedStruct {
                len: Some(Len { byte: 20, bit: 0 }),
                tokens: quote! {},
            },
        );
        let done_fields = vec![];

        let ctx = FieldCtx::new(field, Some(Len { byte: 0, bit: 0 }), &done_fields, &done);

        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 20, bit: 0 }));
        assert_tokens_eq(
            result.field_getter,
            r#"
            pub fn header(&self) -> Header<'_> {
                Header::parse(&self.data[0usize..]).unwrap().0
            }
            "#,
        );
    }

    #[test]
    #[should_panic(expected = "not yet implemented")]
    fn test_field_no_start_offset_no_prev_panics() {
        let dsl = "struct Foo { orphan: u8 }";
        let s = parse_single_struct(dsl);
        let field = extract_field(&s);

        let done = HashMap::<&str, GeneratedStruct>::new();
        let done_fields = vec![];

        let ctx = FieldCtx::new(field, None, &done_fields, &done);

        let _ = ctx.generate();
    }
}

#[cfg(test)]
mod struct_ctx_tests {
    use std::collections::HashMap;

    use binparse::Len;
    use quote::quote;

    use crate::struct_::{GeneratedStruct, StructCtx};

    use super::test_utils::{assert_tokens_eq, parse_single_struct};

    #[test]
    fn test_simple_struct_single_field() {
        let dsl = "struct Simple { value: u32 }";
        let struct_ast = parse_single_struct(dsl);
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = StructCtx::new(&struct_ast, &done);
        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 4, bit: 0 }));
        assert_tokens_eq(
            result.tokens,
            r#"
            pub struct Simple<'a> {
                data: &'a [u8],
            }
            impl<'a> Simple<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.value_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                pub fn value(&self) -> u32 {
                    u32::from_ne_bytes(self.data[0usize..4usize].try_into().unwrap())
                }
                pub fn value_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 4usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_struct_multiple_primitives() {
        let dsl = "struct Header { a: u8, b: u16, c: u8 }";
        let struct_ast = parse_single_struct(dsl);
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = StructCtx::new(&struct_ast, &done);
        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 4, bit: 0 }));
        assert_tokens_eq(
            result.tokens,
            r#"
            pub struct Header<'a> {
                data: &'a [u8],
            }
            impl<'a> Header<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.c_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                pub fn a(&self) -> u8 {
                    u8::from_ne_bytes(self.data[0usize..1usize].try_into().unwrap())
                }
                pub fn a_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 1usize,
                        bit: 0usize,
                    }
                }
                pub fn b(&self) -> u16 {
                    u16::from_ne_bytes(self.data[1usize..3usize].try_into().unwrap())
                }
                pub fn b_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 3usize,
                        bit: 0usize,
                    }
                }
                pub fn c(&self) -> u8 {
                    u8::from_ne_bytes(self.data[3usize..4usize].try_into().unwrap())
                }
                pub fn c_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 4usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_struct_with_bitfields() {
        let dsl = "struct Flags { version: b<4>, ihl: b<4> }";
        let struct_ast = parse_single_struct(dsl);
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = StructCtx::new(&struct_ast, &done);
        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 1, bit: 0 }));
        assert_tokens_eq(
            result.tokens,
            r#"
            pub struct Flags<'a> {
                data: &'a [u8],
            }
            impl<'a> Flags<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.ihl_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                #[allow(clippy::identity_op)]
                pub fn version(&self) -> u8 {
                    (self.data[0usize] >> 0usize) & 15u8
                }
                pub fn version_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 0usize,
                        bit: 4usize,
                    }
                }
                #[allow(clippy::identity_op)]
                pub fn ihl(&self) -> u8 {
                    (self.data[0usize] >> 4usize) & 15u8
                }
                pub fn ihl_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 1usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_struct_with_nested_struct() {
        let dsl = "struct Outer { prefix: u8, inner: Inner }";
        let struct_ast = parse_single_struct(dsl);

        let mut done = HashMap::new();
        done.insert(
            "Inner",
            GeneratedStruct {
                len: Some(Len { byte: 4, bit: 0 }),
                tokens: quote! {},
            },
        );

        let ctx = StructCtx::new(&struct_ast, &done);
        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 5, bit: 0 }));
        assert_tokens_eq(
            result.tokens,
            r#"
            pub struct Outer<'a> {
                data: &'a [u8],
            }
            impl<'a> Outer<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.inner_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                pub fn prefix(&self) -> u8 {
                    u8::from_ne_bytes(self.data[0usize..1usize].try_into().unwrap())
                }
                pub fn prefix_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 1usize,
                        bit: 0usize,
                    }
                }
                pub fn inner(&self) -> Inner<'_> {
                    Inner::parse(&self.data[1usize..]).unwrap().0
                }
                pub fn inner_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 5usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_struct_with_array() {
        let dsl = "struct WithArray { data: [u8; 8] }";
        let struct_ast = parse_single_struct(dsl);
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = StructCtx::new(&struct_ast, &done);
        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 8, bit: 0 }));
        assert_tokens_eq(
            result.tokens,
            r#"
            pub struct WithArray<'a> {
                data: &'a [u8],
            }
            impl<'a> WithArray<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.data_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                pub fn data(&self) -> impl Iterator<Item = u8> + '_ {
                    (0..8usize).map(move |i| {
                        let offset = 0usize + i * 1usize;
                        u8::from_ne_bytes(self.data[offset..offset + 1usize].try_into().unwrap())
                    })
                }
                pub fn data_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 8usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_struct_with_concat() {
        let dsl = "struct WithConcat { fragment_offset: concat(b<5>, b<3>) }";
        let struct_ast = parse_single_struct(dsl);
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = StructCtx::new(&struct_ast, &done);
        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 1, bit: 0 }));
        assert_tokens_eq(
            result.tokens,
            r#"
            pub struct WithConcat<'a> {
                data: &'a [u8],
            }
            impl<'a> WithConcat<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.fragment_offset_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                #[allow(clippy::identity_op)]
                pub fn fragment_offset_0(&self) -> u8 {
                    (self.data[0usize] >> 0usize) & 31u8
                }
                #[allow(clippy::identity_op)]
                pub fn fragment_offset_1(&self) -> u8 {
                    (self.data[0usize] >> 5usize) & 7u8
                }
                pub fn fragment_offset(&self) -> (u8, u8) {
                    (self.fragment_offset_0(), self.fragment_offset_1())
                }
                pub fn fragment_offset_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 1usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_empty_struct() {
        let dsl = "struct Empty {}";
        let struct_ast = parse_single_struct(dsl);
        let done = HashMap::<&str, GeneratedStruct>::new();

        let ctx = StructCtx::new(&struct_ast, &done);
        let result = ctx.generate().unwrap();

        assert_eq!(result.len, Some(Len { byte: 0, bit: 0 }));
        assert_tokens_eq(
            result.tokens,
            r#"
            pub struct Empty<'a> {
                data: &'a [u8],
            }
            impl<'a> Empty<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    Ok((Self { data }, data))
                }
            }
            "#,
        );
    }
}

#[cfg(test)]
mod codegen_tests {
    use crate::CodeGen;

    use super::test_utils::assert_tokens_eq;

    fn generate(dsl: &str) -> String {
        let ast = binparse_dsl_parse::parse_str(dsl).unwrap();
        CodeGen::generate(&ast).unwrap()
    }

    #[test]
    fn test_codegen_simple_struct() {
        let code = generate(
            r#"
            struct Simple {
                value: u32,
            }
            "#,
        );

        assert_tokens_eq(
            code.parse().unwrap(),
            r#"
            pub struct Simple<'a> {
                data: &'a [u8],
            }
            impl<'a> Simple<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.value_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                pub fn value(&self) -> u32 {
                    u32::from_ne_bytes(self.data[0usize..4usize].try_into().unwrap())
                }
                pub fn value_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 4usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_codegen_struct_with_bitfields() {
        let code = generate(
            r#"
            struct IpFlags {
                version: b<4>,
                ihl: b<4>,
                dscp: b<6>,
                ecn: b<2>,
            }
            "#,
        );

        assert_tokens_eq(
            code.parse().unwrap(),
            r#"
            pub struct IpFlags<'a> {
                data: &'a [u8],
            }
            impl<'a> IpFlags<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.ecn_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                #[allow(clippy::identity_op)]
                pub fn version(&self) -> u8 {
                    (self.data[0usize] >> 0usize) & 15u8
                }
                pub fn version_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 0usize,
                        bit: 4usize,
                    }
                }
                #[allow(clippy::identity_op)]
                pub fn ihl(&self) -> u8 {
                    (self.data[0usize] >> 4usize) & 15u8
                }
                pub fn ihl_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 1usize,
                        bit: 0usize,
                    }
                }
                #[allow(clippy::identity_op)]
                pub fn dscp(&self) -> u8 {
                    (self.data[1usize] >> 0usize) & 63u8
                }
                pub fn dscp_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 1usize,
                        bit: 6usize,
                    }
                }
                #[allow(clippy::identity_op)]
                pub fn ecn(&self) -> u8 {
                    (self.data[1usize] >> 6usize) & 3u8
                }
                pub fn ecn_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 2usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_codegen_nested_structs() {
        let code = generate(
            r#"
            struct Inner {
                a: u8,
                b: u8,
            }

            struct Outer {
                prefix: u16,
                inner: Inner,
                suffix: u16,
            }
            "#,
        );

        assert_tokens_eq(
            code.parse().unwrap(),
            r#"
            pub struct Inner<'a> {
                data: &'a [u8],
            }
            impl<'a> Inner<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.b_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                pub fn a(&self) -> u8 {
                    u8::from_ne_bytes(self.data[0usize..1usize].try_into().unwrap())
                }
                pub fn a_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 1usize,
                        bit: 0usize,
                    }
                }
                pub fn b(&self) -> u8 {
                    u8::from_ne_bytes(self.data[1usize..2usize].try_into().unwrap())
                }
                pub fn b_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 2usize,
                        bit: 0usize,
                    }
                }
            }
            pub struct Outer<'a> {
                data: &'a [u8],
            }
            impl<'a> Outer<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.suffix_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                pub fn prefix(&self) -> u16 {
                    u16::from_ne_bytes(self.data[0usize..2usize].try_into().unwrap())
                }
                pub fn prefix_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 2usize,
                        bit: 0usize,
                    }
                }
                pub fn inner(&self) -> Inner<'_> {
                    Inner::parse(&self.data[2usize..]).unwrap().0
                }
                pub fn inner_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 4usize,
                        bit: 0usize,
                    }
                }
                pub fn suffix(&self) -> u16 {
                    u16::from_ne_bytes(self.data[4usize..6usize].try_into().unwrap())
                }
                pub fn suffix_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 6usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_codegen_array_field() {
        let code = generate(
            r#"
            struct WithArray {
                count: u8,
                data: [u32; 4],
            }
            "#,
        );

        assert_tokens_eq(
            code.parse().unwrap(),
            r#"
            pub struct WithArray<'a> {
                data: &'a [u8],
            }
            impl<'a> WithArray<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.data_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                pub fn count(&self) -> u8 {
                    u8::from_ne_bytes(self.data[0usize..1usize].try_into().unwrap())
                }
                pub fn count_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 1usize,
                        bit: 0usize,
                    }
                }
                pub fn data(&self) -> impl Iterator<Item = u32> + '_ {
                    (0..4usize).map(move |i| {
                        let offset = 1usize + i * 4usize;
                        u32::from_ne_bytes(self.data[offset..offset + 4usize].try_into().unwrap())
                    })
                }
                pub fn data_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 17usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_codegen_concat_field() {
        let code = generate(
            r#"
            struct WithConcat {
                flags: b<3>,
                fragment_offset: concat(b<5>, u8),
            }
            "#,
        );

        assert_tokens_eq(
            code.parse().unwrap(),
            r#"
            pub struct WithConcat<'a> {
                data: &'a [u8],
            }
            impl<'a> WithConcat<'a> {
                pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                    let me = Self { data };
                    let len = me.fragment_offset_end_offset();
                    if len.bit != 0 {
                        return Err(binparse::ParseError::UnalignedLength(len));
                    }
                    if data.len() < len.byte {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected: len.byte,
                            got: data.len(),
                        });
                    }
                    Ok((me, &data[len.byte..]))
                }
                #[allow(clippy::identity_op)]
                pub fn flags(&self) -> u8 {
                    (self.data[0usize] >> 0usize) & 7u8
                }
                pub fn flags_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 0usize,
                        bit: 3usize,
                    }
                }
                #[allow(clippy::identity_op)]
                pub fn fragment_offset_0(&self) -> u8 {
                    (self.data[0usize] >> 3usize) & 31u8
                }
                pub fn fragment_offset_1(&self) -> u8 {
                    u8::from_ne_bytes(self.data[1usize..2usize].try_into().unwrap())
                }
                pub fn fragment_offset(&self) -> (u8, u8) {
                    (self.fragment_offset_0(), self.fragment_offset_1())
                }
                pub fn fragment_offset_end_offset(&self) -> binparse::Len {
                    binparse::Len {
                        byte: 2usize,
                        bit: 0usize,
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn test_codegen_duplicate_struct_error() {
        let result = binparse_dsl_parse::parse_str(
            r#"
            struct Dup {
                a: u8,
            }
            struct Dup {
                b: u16,
            }
            "#,
        )
        .and_then(|ast| CodeGen::generate(&ast).map_err(|e| e.to_string()));

        assert!(result.is_err());
    }

    #[test]
    fn test_codegen_complex_ip_header() {
        let code = generate(
            r#"
            struct IpAddr {
                a: u8,
                b: u8,
                c: u8,
                d: u8,
            }

            struct Header {
                version: b<4>,
                ihl: b<4>,
                dscp: b<6>,
                ecn: b<2>,
                total_length: u16,
                id: u16,
                flags: b<3>,
                fragment_offset: concat(b<5>, u8),
                ttl: u8,
                protocol: u8,
                header_checksum: u16,
                src: IpAddr,
                dst: IpAddr,
            }
            "#,
        );

        assert!(code.contains("pub struct IpAddr<'a>"));
        assert!(code.contains("pub struct Header<'a>"));
        assert!(code.contains("pub fn version(&self) -> u8"));
        assert!(code.contains("pub fn src(&self) -> IpAddr<'_>"));
        assert!(code.contains("pub fn dst(&self) -> IpAddr<'_>"));
        assert!(code.contains("pub fn fragment_offset(&self) -> (u8, u8)"));
    }
}
