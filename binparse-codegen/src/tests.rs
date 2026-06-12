use crate::{CodeGen, Error};

fn generate(dsl: &str) -> String {
    let ast = binparse_dsl_parse::parse_str(dsl).expect("failed to parse DSL");
    CodeGen::generate(&ast).expect("failed to generate code")
}

fn generate_err(dsl: &str) -> Error {
    let ast = binparse_dsl_parse::parse_str(dsl).expect("failed to parse DSL");
    CodeGen::generate(&ast).expect_err("expected codegen to fail")
}

fn normalized_items(code: &str) -> Vec<String> {
    let file: syn::File = syn::parse_str(code).expect("failed to parse code as Rust");
    let mut items = file
        .items
        .into_iter()
        .map(|item| {
            prettyplease::unparse(&syn::File {
                shebang: None,
                attrs: vec![],
                items: vec![item],
            })
        })
        .collect::<Vec<_>>();
    items.sort();
    items
}

fn assert_generated_eq(dsl: &str, expected: &str) {
    let actual = normalized_items(&generate(dsl));
    let expected = normalized_items(expected);
    if actual != expected {
        panic!(
            "generated code does not match expected\n\n--- expected ---\n{}\n--- actual ---\n{}",
            expected.join("\n"),
            actual.join("\n")
        );
    }
}

#[test]
fn golden_empty_struct() {
    assert_generated_eq(
        "struct Empty {}",
        r#"
        pub struct Empty<'a> {
            #[allow(dead_code)]
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

#[test]
fn golden_simple_primitive() {
    assert_generated_eq(
        "struct Simple { value: u32 }",
        r#"
        pub struct Simple<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Simple<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.value_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
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
            #[allow(clippy::identity_op)]
            pub fn value(&self) -> u32 {
                u32::from_be_bytes(self.data[0usize..4usize].try_into().unwrap())
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
fn golden_bitfields() {
    assert_generated_eq(
        "struct IpFlags { version: b<4>, ihl: b<4>, dscp: b<6>, ecn: b<2> }",
        r#"
        pub struct IpFlags<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> IpFlags<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.version_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.ihl_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.dscp_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.ecn_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
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
fn golden_cross_byte_bitfield() {
    assert_generated_eq(
        "struct Cross { a: b<5>, b: b<6>, c: b<5> }",
        r#"
        pub struct Cross<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Cross<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.a_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.b_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.c_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
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
            #[allow(clippy::identity_op)]
            pub fn a(&self) -> u8 {
                (self.data[0usize] >> 0usize) & 31u8
            }
            pub fn a_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 0usize,
                    bit: 5usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn b(&self) -> u8 {
                {
                    let first_part = (self.data[0usize] >> 5usize) & 7u8;
                    let second_part = self.data[1usize] & 7u8;
                    first_part | (second_part << 3usize)
                }
            }
            pub fn b_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 1usize,
                    bit: 3usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn c(&self) -> u8 {
                (self.data[1usize] >> 3usize) & 31u8
            }
            pub fn c_end_offset(&self) -> binparse::Len {
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
fn golden_nested_structs() {
    assert_generated_eq(
        "struct Inner { a: u8, b: u8 } struct Outer { prefix: u16, inner: Inner, suffix: u16 }",
        r#"
        pub struct Inner<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Inner<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.a_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.b_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
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
            #[allow(clippy::identity_op)]
            pub fn a(&self) -> u8 {
                self.data[0usize]
            }
            pub fn a_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn b(&self) -> u8 {
                self.data[1usize]
            }
            pub fn b_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 2usize,
                    bit: 0usize,
                }
            }
        }
        pub struct Outer<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Outer<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.prefix_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.inner_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.suffix_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
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
            #[allow(clippy::identity_op)]
            pub fn prefix(&self) -> u16 {
                u16::from_be_bytes(self.data[0usize..2usize].try_into().unwrap())
            }
            pub fn prefix_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 2usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn inner(&self) -> ::binparse::ParseResult<Inner<'_>> {
                Inner::parse(&self.data[2usize..]).map(|(value, _)| value)
            }
            pub fn inner_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 4usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn suffix(&self) -> u16 {
                u16::from_be_bytes(self.data[4usize..6usize].try_into().unwrap())
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
fn golden_fixed_array() {
    assert_generated_eq(
        "struct WithArray { count: u8, data: [u32; 4] }",
        r#"
        #[allow(non_camel_case_types)]
        pub struct data_Iterator<'a> {
            idx: usize,
            count: usize,
            data: &'a [u8],
        }
        impl<'a> ::std::iter::Iterator for data_Iterator<'a> {
            type Item = ::binparse::ParseResult<u32>;
            fn next(&mut self) -> std::option::Option<Self::Item> {
                if self.idx == self.count {
                    return None;
                }
                self.idx += 1;
                let value = u32::from_be_bytes(self.data[..4usize].try_into().unwrap());
                self.data = &self.data[4usize..];
                Some(Ok(value))
            }
        }
        pub struct WithArray<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> WithArray<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.count_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.data_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
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
            #[allow(clippy::identity_op)]
            pub fn count(&self) -> u8 {
                self.data[0usize]
            }
            pub fn count_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn data(&self) -> ::binparse::ParseResult<data_Iterator<'_>> {
                Ok(data_Iterator {
                    idx: 0,
                    count: 4usize,
                    data: &self.data[1usize..],
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
fn golden_expression_sized_array() {
    assert_generated_eq(
        "struct ExprArray { n: u8, items: [u16; n * 2] }",
        r#"
        #[allow(non_camel_case_types)]
        pub struct items_Iterator<'a> {
            idx: usize,
            count: usize,
            data: &'a [u8],
        }
        impl<'a> ::std::iter::Iterator for items_Iterator<'a> {
            type Item = ::binparse::ParseResult<u16>;
            fn next(&mut self) -> std::option::Option<Self::Item> {
                if self.idx == self.count {
                    return None;
                }
                self.idx += 1;
                let value = u16::from_be_bytes(self.data[..2usize].try_into().unwrap());
                self.data = &self.data[2usize..];
                Some(Ok(value))
            }
        }
        pub struct ExprArray<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> ExprArray<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.n_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.items_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                let len = me.items_end_offset();
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
            pub fn n(&self) -> u8 {
                self.data[0usize]
            }
            pub fn n_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn items(&self) -> ::binparse::ParseResult<items_Iterator<'_>> {
                Ok(items_Iterator {
                    idx: 0,
                    count: (self.n() as usize * 2usize) as usize,
                    data: &self.data[1usize..],
                })
            }
            pub fn items_end_offset(&self) -> binparse::Len {
                ::binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
                    + ({
                        ::binparse::Len {
                            byte: 2usize * ((self.n() as usize * 2usize) as usize),
                            bit: 0,
                        }
                    })
            }
        }
        "#,
    );
}

#[test]
fn golden_struct_ref_array() {
    assert_generated_eq(
        "struct Inner { a: u8 } struct StructArray { count: u8, items: [Inner; count] }",
        r#"
        #[allow(non_camel_case_types)]
        pub struct items_Iterator<'a> {
            idx: usize,
            count: usize,
            data: &'a [u8],
        }
        impl<'a> ::std::iter::Iterator for items_Iterator<'a> {
            type Item = ::binparse::ParseResult<Inner<'a>>;
            fn next(&mut self) -> std::option::Option<Self::Item> {
                if self.idx == self.count {
                    return None;
                }
                self.idx += 1;
                match Inner::parse(self.data) {
                    Ok((value, rem)) => {
                        self.data = rem;
                        Some(Ok(value))
                    }
                    Err(error) => Some(Err(error)),
                }
            }
        }
        pub struct StructArray<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> StructArray<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.count_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.items_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                let len = me.items_end_offset();
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
            pub fn count(&self) -> u8 {
                self.data[0usize]
            }
            pub fn count_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn items(&self) -> ::binparse::ParseResult<items_Iterator<'_>> {
                Ok(items_Iterator {
                    idx: 0,
                    count: self.count() as usize,
                    data: &self.data[1usize..],
                })
            }
            pub fn items_end_offset(&self) -> binparse::Len {
                ::binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
                    + ({
                        ::binparse::Len {
                            byte: 1usize * (self.count() as usize),
                            bit: 0,
                        }
                    })
            }
        }
        pub struct Inner<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Inner<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.a_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                let len = me.a_end_offset();
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
            pub fn a(&self) -> u8 {
                self.data[0usize]
            }
            pub fn a_end_offset(&self) -> binparse::Len {
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
fn golden_bitfield_array() {
    assert_generated_eq(
        "struct BitArray { nibbles: [b<4>; 4] }",
        r#"
        #[allow(non_camel_case_types)]
        pub struct nibbles_Iterator<'a> {
            idx: usize,
            count: usize,
            data: &'a [u8],
            bit_offset: usize,
        }
        impl<'a> ::std::iter::Iterator for nibbles_Iterator<'a> {
            type Item = ::binparse::ParseResult<u8>;
            fn next(&mut self) -> std::option::Option<Self::Item> {
                if self.idx == self.count {
                    return None;
                }
                self.idx += 1;
                let byte_idx = self.bit_offset / 8;
                let bit_idx = self.bit_offset % 8;
                let value = if bit_idx + 4usize <= 8 {
                    let mask = (1u8 << 4usize) - 1;
                    (self.data[byte_idx] >> bit_idx) & mask
                } else {
                    let bits_in_first = 8 - bit_idx;
                    let bits_in_second = 4usize - bits_in_first;
                    let first_mask = (1u8 << bits_in_first) - 1;
                    let second_mask = (1u8 << bits_in_second) - 1;
                    let first_part = (self.data[byte_idx] >> bit_idx) & first_mask;
                    let second_part = self.data[byte_idx + 1] & second_mask;
                    first_part | (second_part << bits_in_first)
                };
                self.bit_offset += 4usize;
                Some(Ok(value))
            }
        }
        pub struct BitArray<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> BitArray<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.nibbles_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                let len = me.nibbles_end_offset();
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
            pub fn nibbles(&self) -> ::binparse::ParseResult<nibbles_Iterator<'_>> {
                Ok(nibbles_Iterator {
                    idx: 0,
                    count: 4usize,
                    data: &self.data[0usize..],
                    bit_offset: 0,
                })
            }
            pub fn nibbles_end_offset(&self) -> binparse::Len {
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
fn golden_concat() {
    assert_generated_eq(
        "struct WithConcat { flags: b<3>, fragment_offset: concat(b<5>, u8) }",
        r#"
        pub struct WithConcat<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> WithConcat<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.flags_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.fragment_offset_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
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
            #[allow(clippy::identity_op)]
            pub fn fragment_offset_1(&self) -> u8 {
                self.data[1usize]
            }
            #[allow(clippy::identity_op)]
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
fn golden_union_single_discriminant() {
    assert_generated_eq(
        r#"struct Packet {
            ty: u8,
            payload: union(ty) {
                1 => Echo { id: u16 },
                _ => Unknown { },
            },
        }"#,
        r#"
        #[allow(non_camel_case_types)]
        pub struct Packet_payload_Echo<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Packet_payload_Echo<'a> {
            #[allow(clippy::identity_op)]
            pub fn id(&self) -> u16 {
                u16::from_be_bytes(self.data[0usize..2usize].try_into().unwrap())
            }
            pub fn id_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 2usize,
                    bit: 0usize,
                }
            }
        }
        #[allow(non_camel_case_types)]
        pub struct Packet_payload_Unknown<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Packet_payload_Unknown<'a> {}
        #[allow(non_camel_case_types)]
        pub enum Packet_payload<'a> {
            Echo(Packet_payload_Echo<'a>),
            Unknown(Packet_payload_Unknown<'a>),
        }
        pub struct Packet<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Packet<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.ty_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.payload_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                let len = me.payload_end_offset();
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
            pub fn ty(&self) -> u8 {
                self.data[0usize]
            }
            pub fn ty_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn payload(&self) -> Packet_payload<'_> {
                match self.ty() {
                    1 => {
                        Packet_payload::Echo(Packet_payload_Echo {
                            data: &self.data[1usize..],
                        })
                    }
                    _ => {
                        Packet_payload::Unknown(Packet_payload_Unknown {
                            data: &self.data[1usize..],
                        })
                    }
                }
            }
            pub fn payload_end_offset(&self) -> binparse::Len {
                ::binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
                    + ({
                        match self.ty() {
                            1 => {
                                ::binparse::Len {
                                    byte: 2usize,
                                    bit: 0,
                                }
                            }
                            _ => {
                                ::binparse::Len {
                                    byte: 0usize,
                                    bit: 0,
                                }
                            }
                        }
                    })
            }
        }
        "#,
    );
}

#[test]
fn golden_union_tuple_discriminant_with_multiple_matchers() {
    assert_generated_eq(
        r#"struct Packet {
            ty: u8,
            code: u8,
            payload: union(ty, code) {
                (0, 0) | (0, 8) => Echo { id: u16 },
                _ => Unknown { },
            },
        }"#,
        r#"
        #[allow(non_camel_case_types)]
        pub struct Packet_payload_Echo<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Packet_payload_Echo<'a> {
            #[allow(clippy::identity_op)]
            pub fn id(&self) -> u16 {
                u16::from_be_bytes(self.data[0usize..2usize].try_into().unwrap())
            }
            pub fn id_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 2usize,
                    bit: 0usize,
                }
            }
        }
        #[allow(non_camel_case_types)]
        pub struct Packet_payload_Unknown<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Packet_payload_Unknown<'a> {}
        #[allow(non_camel_case_types)]
        pub enum Packet_payload<'a> {
            Echo(Packet_payload_Echo<'a>),
            Unknown(Packet_payload_Unknown<'a>),
        }
        pub struct Packet<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> Packet<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.ty_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.code_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.payload_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                let len = me.payload_end_offset();
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
            pub fn ty(&self) -> u8 {
                self.data[0usize]
            }
            pub fn ty_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn code(&self) -> u8 {
                self.data[1usize]
            }
            pub fn code_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 2usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn payload(&self) -> Packet_payload<'_> {
                match (self.ty(), self.code()) {
                    (0, 0) | (0, 8) => {
                        Packet_payload::Echo(Packet_payload_Echo {
                            data: &self.data[2usize..],
                        })
                    }
                    _ => {
                        Packet_payload::Unknown(Packet_payload_Unknown {
                            data: &self.data[2usize..],
                        })
                    }
                }
            }
            pub fn payload_end_offset(&self) -> binparse::Len {
                ::binparse::Len {
                    byte: 2usize,
                    bit: 0usize,
                }
                    + ({
                        match (self.ty(), self.code()) {
                            (0, 0) | (0, 8) => {
                                ::binparse::Len {
                                    byte: 2usize,
                                    bit: 0,
                                }
                            }
                            _ => {
                                ::binparse::Len {
                                    byte: 0usize,
                                    bit: 0,
                                }
                            }
                        }
                    })
            }
        }
        "#,
    );
}

#[test]
fn golden_endian_attributes() {
    assert_generated_eq(
        r#"@endian(little)
        struct LittlePacket {
            header: u32,
            @endian(big) mixed: u16,
            data: u8,
        }"#,
        r#"
        pub struct LittlePacket<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> LittlePacket<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.header_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.mixed_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.data_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
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
            #[allow(clippy::identity_op)]
            pub fn header(&self) -> u32 {
                u32::from_le_bytes(self.data[0usize..4usize].try_into().unwrap())
            }
            pub fn header_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 4usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn mixed(&self) -> u16 {
                u16::from_be_bytes(self.data[4usize..6usize].try_into().unwrap())
            }
            pub fn mixed_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 6usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn data(&self) -> u8 {
                self.data[6usize]
            }
            pub fn data_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 7usize,
                    bit: 0usize,
                }
            }
        }
        "#,
    );
}

#[test]
fn golden_fixed_hook() {
    assert_generated_eq(
        r#"struct WithFixedHook {
            prefix: u8,
            @hook(double_it, u32)
            value: u16,
            suffix: u8,
        }"#,
        r#"
        pub struct WithFixedHook<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> WithFixedHook<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.prefix_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.value_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.suffix_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
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
            #[allow(clippy::identity_op)]
            pub fn prefix(&self) -> u8 {
                self.data[0usize]
            }
            pub fn prefix_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn value(&self) -> u32 {
                double_it(u16::from_be_bytes(self.data[1usize..3usize].try_into().unwrap()))
            }
            pub fn value_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 3usize,
                    bit: 0usize,
                }
            }
            #[allow(clippy::identity_op)]
            pub fn suffix(&self) -> u8 {
                self.data[3usize]
            }
            pub fn suffix_end_offset(&self) -> binparse::Len {
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
fn golden_vla_hook() {
    assert_generated_eq(
        r#"struct WithVlaHook {
            len: u8,
            @hook(parse_cstring, String)
            name: [u8],
        }"#,
        r#"
        pub struct WithVlaHook<'a> {
            #[allow(dead_code)]
            data: &'a [u8],
        }
        impl<'a> WithVlaHook<'a> {
            pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
                let me = Self { data };
                {
                    let len = me.len_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                {
                    let len = me.name_end_offset();
                    let expected = len.byte + usize::from(len.bit > 0);
                    if data.len() < expected {
                        return Err(binparse::ParseError::NotEnoughData {
                            expected,
                            got: data.len(),
                        });
                    }
                }
                let len = me.name_end_offset();
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
            pub fn len(&self) -> u8 {
                self.data[0usize]
            }
            pub fn len_end_offset(&self) -> binparse::Len {
                binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
            }
            fn name_raw(&self) -> (String, usize) {
                parse_cstring(&self.data[self.len_end_offset().byte..])
            }
            pub fn name(&self) -> String {
                self.name_raw().0
            }
            pub fn name_end_offset(&self) -> binparse::Len {
                ::binparse::Len {
                    byte: 1usize,
                    bit: 0usize,
                }
                    + ({
                        binparse::Len {
                            byte: self.name_raw().1,
                            bit: 0,
                        }
                    })
            }
        }
        "#,
    );
}

#[test]
fn duplicate_struct_is_rejected() {
    let err = generate_err("struct Dup { a: u8 } struct Dup { b: u16 }");
    assert!(matches!(err, Error::DuplicateStruct { .. }));
}

#[test]
fn endian_on_u8_is_rejected() {
    let err = generate_err("struct Foo { @endian(little) a: u8 }");
    assert!(err.to_string().contains("@endian cannot be applied to u8"));
}

#[test]
fn endian_on_bitfield_is_rejected() {
    let err = generate_err("struct Foo { @endian(little) a: b<4> }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to bitfields")
    );
}

#[test]
fn endian_on_struct_ref_is_rejected() {
    let err = generate_err("struct Inner { x: u8 } struct Foo { @endian(big) inner: Inner }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to struct ref")
    );
}

#[test]
fn invalid_endian_value_is_rejected() {
    let err = generate_err("struct Foo { @endian(middle) a: u16 }");
    assert!(err.to_string().contains("@endian argument must be"));
}

#[test]
fn endian_wrong_arg_count_is_rejected() {
    let err = generate_err("struct Foo { @endian(big, little) a: u16 }");
    assert!(err.to_string().contains("requires exactly 1 argument"));
}

#[test]
fn array_size_unknown_field_is_rejected() {
    let err = generate_err("struct Foo { items: [u16; nope] }");
    assert!(
        err.to_string()
            .contains("array size path references unknown field 'nope'")
    );
}

#[test]
fn array_size_non_numeric_field_is_rejected() {
    let err = generate_err("struct Inner { x: u8 } struct Foo { inner: Inner, items: [u8; inner] }");
    assert!(
        err.to_string()
            .contains("array size path must reference a primitive or bitfield")
    );
}

#[test]
fn union_unknown_argument_is_rejected() {
    let err = generate_err(
        r#"struct Foo {
            payload: union(kind) {
                1 => A { x: u8 },
                _ => B { },
            },
        }"#,
    );
    assert!(err.to_string().contains("argument not found kind"));
}
