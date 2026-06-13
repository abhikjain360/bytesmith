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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let children = ::std::vec::Vec::new();
        let mut root = ::binparse::FieldNode::new(
                "Empty",
                "Empty",
                0usize..0usize,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let _ = Self { data };
        let children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut root = ::binparse::FieldNode::new(
                "Empty",
                "Empty",
                0usize..0usize,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
}
impl<'a> ::binparse::Dissect<'a> for Empty<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Empty::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Empty::handoff(self)
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
        me.value_fatal_check()?;
        me.value_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.value_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Simple",
                "Simple",
                0usize..self.value_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.value_fatal_check() {
                Err(error) => {
                    let start = me.value_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "value",
                                    "u32",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.value_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.value_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "value",
                                            "u32",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.value_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Simple",
                "Simple",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn value_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.value_bit_range();
        ::binparse::FieldNode::new(
            "value",
            "u32",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.value())),
        )
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
    pub fn value_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn value_bit_range(&self) -> ::core::ops::Range<usize> {
        self.value_start_offset().bits()..self.value_end_offset().bits()
    }
    #[allow(dead_code)]
    fn value_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.value_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn value_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Simple<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Simple::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Simple::handoff(self)
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
        me.version_fatal_check()?;
        me.version_recoverable_check()?;
        me.ihl_fatal_check()?;
        me.ihl_recoverable_check()?;
        me.dscp_fatal_check()?;
        me.dscp_recoverable_check()?;
        me.ecn_fatal_check()?;
        me.ecn_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.version_present_node());
        }
        {
            let me = self;
            children.push(me.ihl_present_node());
        }
        {
            let me = self;
            children.push(me.dscp_present_node());
        }
        {
            let me = self;
            children.push(me.ecn_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "IpFlags",
                "IpFlags",
                0usize..self.ecn_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.version_fatal_check() {
                Err(error) => {
                    let start = me.version_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "version",
                                    "b<4>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.version_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.version_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "version",
                                            "b<4>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.version_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.ihl_fatal_check() {
                Err(error) => {
                    let start = me.ihl_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "ihl",
                                    "b<4>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.ihl_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.ihl_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "ihl",
                                            "b<4>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.ihl_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.dscp_fatal_check() {
                Err(error) => {
                    let start = me.dscp_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "dscp",
                                    "b<6>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.dscp_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.dscp_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "dscp",
                                            "b<6>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.dscp_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.ecn_fatal_check() {
                Err(error) => {
                    let start = me.ecn_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "ecn",
                                    "b<2>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.ecn_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.ecn_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "ecn",
                                            "b<2>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.ecn_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "IpFlags",
                "IpFlags",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn version_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.version_bit_range();
        ::binparse::FieldNode::new(
            "version",
            "b<4>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.version())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn version(&self) -> u8 {
        (self.data[0usize] >> 4usize) & 15u8
    }
    pub fn version_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 0usize,
            bit: 4usize,
        }
    }
    pub fn version_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn version_bit_range(&self) -> ::core::ops::Range<usize> {
        self.version_start_offset().bits()..self.version_end_offset().bits()
    }
    #[allow(dead_code)]
    fn version_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.version_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn version_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn ihl_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.ihl_bit_range();
        ::binparse::FieldNode::new(
            "ihl",
            "b<4>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.ihl())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn ihl(&self) -> u8 {
        (self.data[0usize] >> 0usize) & 15u8
    }
    pub fn ihl_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
    }
    pub fn ihl_start_offset(&self) -> binparse::Len {
        self.version_end_offset()
    }
    pub fn ihl_bit_range(&self) -> ::core::ops::Range<usize> {
        self.ihl_start_offset().bits()..self.ihl_end_offset().bits()
    }
    #[allow(dead_code)]
    fn ihl_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.ihl_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn ihl_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn dscp_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.dscp_bit_range();
        ::binparse::FieldNode::new(
            "dscp",
            "b<6>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.dscp())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn dscp(&self) -> u8 {
        (self.data[1usize] >> 2usize) & 63u8
    }
    pub fn dscp_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 1usize,
            bit: 6usize,
        }
    }
    pub fn dscp_start_offset(&self) -> binparse::Len {
        self.ihl_end_offset()
    }
    pub fn dscp_bit_range(&self) -> ::core::ops::Range<usize> {
        self.dscp_start_offset().bits()..self.dscp_end_offset().bits()
    }
    #[allow(dead_code)]
    fn dscp_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.dscp_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn dscp_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn ecn_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.ecn_bit_range();
        ::binparse::FieldNode::new(
            "ecn",
            "b<2>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.ecn())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn ecn(&self) -> u8 {
        (self.data[1usize] >> 0usize) & 3u8
    }
    pub fn ecn_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
    }
    pub fn ecn_start_offset(&self) -> binparse::Len {
        self.dscp_end_offset()
    }
    pub fn ecn_bit_range(&self) -> ::core::ops::Range<usize> {
        self.ecn_start_offset().bits()..self.ecn_end_offset().bits()
    }
    #[allow(dead_code)]
    fn ecn_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.ecn_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn ecn_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for IpFlags<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        IpFlags::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        IpFlags::handoff(self)
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
        me.a_fatal_check()?;
        me.a_recoverable_check()?;
        me.b_fatal_check()?;
        me.b_recoverable_check()?;
        me.c_fatal_check()?;
        me.c_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.a_present_node());
        }
        {
            let me = self;
            children.push(me.b_present_node());
        }
        {
            let me = self;
            children.push(me.c_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Cross",
                "Cross",
                0usize..self.c_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.a_fatal_check() {
                Err(error) => {
                    let start = me.a_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "a",
                                    "b<5>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.a_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.a_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "a",
                                            "b<5>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.a_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.b_fatal_check() {
                Err(error) => {
                    let start = me.b_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "b",
                                    "b<6>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.b_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.b_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "b",
                                            "b<6>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.b_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.c_fatal_check() {
                Err(error) => {
                    let start = me.c_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "c",
                                    "b<5>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.c_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.c_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "c",
                                            "b<5>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.c_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Cross",
                "Cross",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn a_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.a_bit_range();
        ::binparse::FieldNode::new(
            "a",
            "b<5>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.a())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn a(&self) -> u8 {
        (self.data[0usize] >> 3usize) & 31u8
    }
    pub fn a_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 0usize,
            bit: 5usize,
        }
    }
    pub fn a_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn a_bit_range(&self) -> ::core::ops::Range<usize> {
        self.a_start_offset().bits()..self.a_end_offset().bits()
    }
    #[allow(dead_code)]
    fn a_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.a_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn a_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn b_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.b_bit_range();
        ::binparse::FieldNode::new(
            "b",
            "b<6>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.b())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn b(&self) -> u8 {
        {
            let first_part = self.data[0usize] & 7u8;
            let second_part = self.data[1usize] >> 5usize;
            (first_part << 3usize) | second_part
        }
    }
    pub fn b_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 1usize,
            bit: 3usize,
        }
    }
    pub fn b_start_offset(&self) -> binparse::Len {
        self.a_end_offset()
    }
    pub fn b_bit_range(&self) -> ::core::ops::Range<usize> {
        self.b_start_offset().bits()..self.b_end_offset().bits()
    }
    #[allow(dead_code)]
    fn b_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.b_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn b_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn c_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.c_bit_range();
        ::binparse::FieldNode::new(
            "c",
            "b<5>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.c())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn c(&self) -> u8 {
        (self.data[1usize] >> 0usize) & 31u8
    }
    pub fn c_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
    }
    pub fn c_start_offset(&self) -> binparse::Len {
        self.b_end_offset()
    }
    pub fn c_bit_range(&self) -> ::core::ops::Range<usize> {
        self.c_start_offset().bits()..self.c_end_offset().bits()
    }
    #[allow(dead_code)]
    fn c_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.c_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn c_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Cross<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Cross::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Cross::handoff(self)
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
        me.a_fatal_check()?;
        me.a_recoverable_check()?;
        me.b_fatal_check()?;
        me.b_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.a_present_node());
        }
        {
            let me = self;
            children.push(me.b_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Inner",
                "Inner",
                0usize..self.b_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.a_fatal_check() {
                Err(error) => {
                    let start = me.a_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "a",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.a_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.a_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "a",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.a_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.b_fatal_check() {
                Err(error) => {
                    let start = me.b_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "b",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.b_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.b_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "b",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.b_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Inner",
                "Inner",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn a_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.a_bit_range();
        ::binparse::FieldNode::new(
            "a",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.a())),
        )
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
    pub fn a_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn a_bit_range(&self) -> ::core::ops::Range<usize> {
        self.a_start_offset().bits()..self.a_end_offset().bits()
    }
    #[allow(dead_code)]
    fn a_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.a_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn a_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn b_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.b_bit_range();
        ::binparse::FieldNode::new(
            "b",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.b())),
        )
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
    pub fn b_start_offset(&self) -> binparse::Len {
        self.a_end_offset()
    }
    pub fn b_bit_range(&self) -> ::core::ops::Range<usize> {
        self.b_start_offset().bits()..self.b_end_offset().bits()
    }
    #[allow(dead_code)]
    fn b_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.b_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn b_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Inner<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Inner::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Inner::handoff(self)
    }
}
pub struct Outer<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Outer<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.prefix_fatal_check()?;
        me.prefix_recoverable_check()?;
        me.inner_fatal_check()?;
        me.inner_recoverable_check()?;
        me.suffix_fatal_check()?;
        me.suffix_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.prefix_present_node());
        }
        {
            let me = self;
            children.push(me.inner_present_node());
        }
        {
            let me = self;
            children.push(me.suffix_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Outer",
                "Outer",
                0usize..self.suffix_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.prefix_fatal_check() {
                Err(error) => {
                    let start = me.prefix_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "prefix",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.prefix_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.prefix_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "prefix",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.prefix_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.inner_fatal_check() {
                Err(error) => {
                    let start = me.inner_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "inner",
                                    "Inner",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.inner_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.inner_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "inner",
                                            "Inner",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.inner_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.suffix_fatal_check() {
                Err(error) => {
                    let start = me.suffix_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "suffix",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.suffix_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.suffix_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "suffix",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.suffix_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Outer",
                "Outer",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn prefix_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.prefix_bit_range();
        ::binparse::FieldNode::new(
            "prefix",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.prefix())),
        )
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
    pub fn prefix_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn prefix_bit_range(&self) -> ::core::ops::Range<usize> {
        self.prefix_start_offset().bits()..self.prefix_end_offset().bits()
    }
    #[allow(dead_code)]
    fn prefix_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.prefix_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn prefix_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn inner_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.inner_bit_range();
        match self.inner() {
            Ok(value) => value.field_tree().renamed("inner").shifted(bit_range.start),
            Err(error) => {
                ::binparse::FieldNode::new(
                        "inner",
                        "Inner",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Error(error))
            }
        }
    }
    #[allow(clippy::identity_op)]
    pub fn inner(&self) -> ::binparse::ParseResult<Inner<'a>> {
        Inner::parse(&self.data[2usize..]).map(|(value, _)| value)
    }
    pub fn inner_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 4usize,
            bit: 0usize,
        }
    }
    pub fn inner_start_offset(&self) -> binparse::Len {
        self.prefix_end_offset()
    }
    pub fn inner_bit_range(&self) -> ::core::ops::Range<usize> {
        self.inner_start_offset().bits()..self.inner_end_offset().bits()
    }
    #[allow(dead_code)]
    fn inner_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.inner_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn inner_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn suffix_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.suffix_bit_range();
        ::binparse::FieldNode::new(
            "suffix",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.suffix())),
        )
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
    pub fn suffix_start_offset(&self) -> binparse::Len {
        self.inner_end_offset()
    }
    pub fn suffix_bit_range(&self) -> ::core::ops::Range<usize> {
        self.suffix_start_offset().bits()..self.suffix_end_offset().bits()
    }
    #[allow(dead_code)]
    fn suffix_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.suffix_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn suffix_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Outer<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Outer::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Outer::handoff(self)
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
pub struct WithArray_data_Iterator<'a> {
    idx: usize,
    count: usize,
    data: &'a [u8],
}
impl<'a> ::std::iter::Iterator for WithArray_data_Iterator<'a> {
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
        me.count_fatal_check()?;
        me.count_recoverable_check()?;
        me.data_fatal_check()?;
        me.data_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.count_present_node());
        }
        {
            let me = self;
            children.push(me.data_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "WithArray",
                "WithArray",
                0usize..self.data_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.count_fatal_check() {
                Err(error) => {
                    let start = me.count_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "count",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.count_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.count_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "count",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.count_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.data_fatal_check() {
                Err(error) => {
                    let start = me.data_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "data",
                                    "[u32]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.data_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.data_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "data",
                                            "[u32]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.data_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "WithArray",
                "WithArray",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn count_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.count_bit_range();
        ::binparse::FieldNode::new(
            "count",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.count())),
        )
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
    pub fn count_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn count_bit_range(&self) -> ::core::ops::Range<usize> {
        self.count_start_offset().bits()..self.count_end_offset().bits()
    }
    #[allow(dead_code)]
    fn count_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.count_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn count_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn data_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.data_bit_range();
        {
            let mut elem_nodes = ::std::vec::Vec::new();
            if let Ok(iter) = self.data() {
                let mut start = bit_range.start;
                for (i, elem) in iter.enumerate() {
                    let end = start.saturating_add(32usize);
                    match elem {
                        Ok(value) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                        i.to_string(),
                                        "u32",
                                        start..end,
                                        ::binparse::Value::UInt(u128::from(value)),
                                    ),
                                )
                        }
                        Err(error) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                            i.to_string(),
                                            "u32",
                                            start..start,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                    }
                    start = end;
                }
            }
            ::binparse::FieldNode::new(
                    "data",
                    "[u32]",
                    bit_range.clone(),
                    ::binparse::Value::Array,
                )
                .with_children(elem_nodes)
        }
    }
    #[allow(clippy::identity_op)]
    pub fn data(&self) -> ::binparse::ParseResult<WithArray_data_Iterator<'a>> {
        Ok(WithArray_data_Iterator {
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
    pub fn data_start_offset(&self) -> binparse::Len {
        self.count_end_offset()
    }
    pub fn data_bit_range(&self) -> ::core::ops::Range<usize> {
        self.data_start_offset().bits()..self.data_end_offset().bits()
    }
    #[allow(dead_code)]
    fn data_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.data_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn data_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for WithArray<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        WithArray::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        WithArray::handoff(self)
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
pub struct ExprArray_items_Iterator<'a> {
    idx: usize,
    count: usize,
    data: &'a [u8],
}
impl<'a> ::std::iter::Iterator for ExprArray_items_Iterator<'a> {
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
        me.n_fatal_check()?;
        me.n_recoverable_check()?;
        me.items_fatal_check()?;
        me.items_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.n_present_node());
        }
        {
            let me = self;
            children.push(me.items_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "ExprArray",
                "ExprArray",
                0usize..self.items_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.n_fatal_check() {
                Err(error) => {
                    let start = me.n_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "n",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.n_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.n_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "n",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.n_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.items_fatal_check() {
                Err(error) => {
                    let start = me.items_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "items",
                                    "[u16]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.items_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.items_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "items",
                                            "[u16]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.items_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "ExprArray",
                "ExprArray",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn n_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.n_bit_range();
        ::binparse::FieldNode::new(
            "n",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.n())),
        )
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
    pub fn n_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn n_bit_range(&self) -> ::core::ops::Range<usize> {
        self.n_start_offset().bits()..self.n_end_offset().bits()
    }
    #[allow(dead_code)]
    fn n_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.n_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn n_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn items_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.items_bit_range();
        {
            let mut elem_nodes = ::std::vec::Vec::new();
            if let Ok(iter) = self.items() {
                let mut start = bit_range.start;
                for (i, elem) in iter.enumerate() {
                    let end = start.saturating_add(16usize);
                    match elem {
                        Ok(value) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                        i.to_string(),
                                        "u16",
                                        start..end,
                                        ::binparse::Value::UInt(u128::from(value)),
                                    ),
                                )
                        }
                        Err(error) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                            i.to_string(),
                                            "u16",
                                            start..start,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                    }
                    start = end;
                }
            }
            ::binparse::FieldNode::new(
                    "items",
                    "[u16]",
                    bit_range.clone(),
                    ::binparse::Value::Array,
                )
                .with_children(elem_nodes)
        }
    }
    #[allow(clippy::identity_op)]
    pub fn items(&self) -> ::binparse::ParseResult<ExprArray_items_Iterator<'a>> {
        Ok(ExprArray_items_Iterator {
            idx: 0,
            count: (self.n() as usize).saturating_mul(2usize),
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
                    byte: 2usize,
                    bit: 0usize,
                } * ((self.n() as usize).saturating_mul(2usize))
            })
    }
    pub fn items_start_offset(&self) -> binparse::Len {
        self.n_end_offset()
    }
    pub fn items_bit_range(&self) -> ::core::ops::Range<usize> {
        self.items_start_offset().bits()..self.items_end_offset().bits()
    }
    #[allow(dead_code)]
    fn items_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.items_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn items_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for ExprArray<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        ExprArray::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        ExprArray::handoff(self)
    }
}

"#,
    );
}

#[test]
fn golden_greedy_rest_array() {
    assert_generated_eq(
        "struct Rest { n: u8, @greedy(unsafe_eof) tail: [u8] }",
        r#"
#[allow(non_camel_case_types)]
pub struct Rest_tail_Iterator<'a> {
    idx: usize,
    count: usize,
    data: &'a [u8],
}
impl<'a> ::std::iter::Iterator for Rest_tail_Iterator<'a> {
    type Item = ::binparse::ParseResult<u8>;
    fn next(&mut self) -> std::option::Option<Self::Item> {
        if self.idx == self.count {
            return None;
        }
        self.idx += 1;
        let value = self.data[0];
        self.data = &self.data[1..];
        Some(Ok(value))
    }
}
pub struct Rest<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Rest<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.n_fatal_check()?;
        me.n_recoverable_check()?;
        me.tail_fatal_check()?;
        me.tail_recoverable_check()?;
        let len = me.tail_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.n_present_node());
        }
        {
            let me = self;
            children.push(me.tail_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Rest",
                "Rest",
                0usize..self.tail_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.n_fatal_check() {
                Err(error) => {
                    let start = me.n_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "n",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.n_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.n_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "n",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.n_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.tail_fatal_check() {
                Err(error) => {
                    let start = me.tail_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "tail",
                                    "[u8]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.tail_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.tail_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "tail",
                                            "[u8]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.tail_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Rest",
                "Rest",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn n_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.n_bit_range();
        ::binparse::FieldNode::new(
            "n",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.n())),
        )
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
    pub fn n_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn n_bit_range(&self) -> ::core::ops::Range<usize> {
        self.n_start_offset().bits()..self.n_end_offset().bits()
    }
    #[allow(dead_code)]
    fn n_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.n_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn n_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn tail_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.tail_bit_range();
        {
            let mut elem_nodes = ::std::vec::Vec::new();
            if let Ok(iter) = self.tail() {
                let mut start = bit_range.start;
                for (i, elem) in iter.enumerate() {
                    let end = start.saturating_add(8usize);
                    match elem {
                        Ok(value) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                        i.to_string(),
                                        "u8",
                                        start..end,
                                        ::binparse::Value::UInt(u128::from(value)),
                                    ),
                                )
                        }
                        Err(error) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                            i.to_string(),
                                            "u8",
                                            start..start,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                    }
                    start = end;
                }
            }
            ::binparse::FieldNode::new(
                    "tail",
                    "[u8]",
                    bit_range.clone(),
                    ::binparse::Value::Array,
                )
                .with_children(elem_nodes)
        }
    }
    #[allow(clippy::identity_op)]
    pub fn tail(&self) -> ::binparse::ParseResult<Rest_tail_Iterator<'a>> {
        Ok(Rest_tail_Iterator {
            idx: 0,
            count: self.data.len().saturating_sub(1usize),
            data: &self.data[1usize..],
        })
    }
    pub fn tail_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
            + ({
                {
                    let start = 1usize;
                    ::binparse::Len {
                        byte: self.data.len().saturating_sub(start),
                        bit: 0,
                    }
                }
            })
    }
    pub fn tail_start_offset(&self) -> binparse::Len {
        self.n_end_offset()
    }
    pub fn tail_bit_range(&self) -> ::core::ops::Range<usize> {
        self.tail_start_offset().bits()..self.tail_end_offset().bits()
    }
    #[allow(dead_code)]
    fn tail_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.tail_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn tail_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Rest<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Rest::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Rest::handoff(self)
    }
}

"#,
    );
}

#[test]
fn golden_until_array() {
    assert_generated_eq(
        "struct CStr { @until(x00) name: [u8], after: u8 }",
        r#"
#[allow(non_camel_case_types)]
pub struct CStr_name_Iterator<'a> {
    idx: usize,
    count: usize,
    data: &'a [u8],
}
impl<'a> ::std::iter::Iterator for CStr_name_Iterator<'a> {
    type Item = ::binparse::ParseResult<u8>;
    fn next(&mut self) -> std::option::Option<Self::Item> {
        if self.idx == self.count {
            return None;
        }
        self.idx += 1;
        let value = self.data[0];
        self.data = &self.data[1..];
        Some(Ok(value))
    }
}
pub struct CStr<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> CStr<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.name_fatal_check()?;
        me.name_recoverable_check()?;
        me.after_fatal_check()?;
        me.after_recoverable_check()?;
        let len = me.after_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.name_present_node());
        }
        {
            let me = self;
            children.push(me.after_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "CStr",
                "CStr",
                0usize..self.after_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.name_fatal_check() {
                Err(error) => {
                    let start = me.name_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "name",
                                    "[u8]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.name_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.name_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "name",
                                            "[u8]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.name_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.after_fatal_check() {
                Err(error) => {
                    let start = me.after_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "after",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.after_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.after_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "after",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.after_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "CStr",
                "CStr",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn name_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.name_bit_range();
        {
            let mut elem_nodes = ::std::vec::Vec::new();
            if let Ok(iter) = self.name() {
                let mut start = bit_range.start;
                for (i, elem) in iter.enumerate() {
                    let end = start.saturating_add(8usize);
                    match elem {
                        Ok(value) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                        i.to_string(),
                                        "u8",
                                        start..end,
                                        ::binparse::Value::UInt(u128::from(value)),
                                    ),
                                )
                        }
                        Err(error) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                            i.to_string(),
                                            "u8",
                                            start..start,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                    }
                    start = end;
                }
            }
            ::binparse::FieldNode::new(
                    "name",
                    "[u8]",
                    bit_range.clone(),
                    ::binparse::Value::Array,
                )
                .with_children(elem_nodes)
        }
    }
    #[allow(clippy::identity_op)]
    pub fn name(&self) -> ::binparse::ParseResult<CStr_name_Iterator<'a>> {
        Ok(CStr_name_Iterator {
            idx: 0,
            count: self.data[0usize..].iter().position(|&b| b == 0u8).unwrap_or(0),
            data: &self.data[0usize..],
        })
    }
    pub fn name_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 0usize,
            bit: 0usize,
        }
            + ({
                {
                    let start = 0usize;
                    let byte = match self
                        .data
                        .get(start..)
                        .and_then(|rest| rest.iter().position(|&b| b == 0u8))
                    {
                        Some(pos) => pos.saturating_add(1),
                        None => self.data.len().saturating_add(1).saturating_sub(start),
                    };
                    ::binparse::Len { byte, bit: 0 }
                }
            })
    }
    pub fn name_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn name_bit_range(&self) -> ::core::ops::Range<usize> {
        self.name_start_offset().bits()..self.name_end_offset().bits()
    }
    #[allow(dead_code)]
    fn name_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.name_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn name_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn after_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.after_bit_range();
        ::binparse::FieldNode::new(
            "after",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.after())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn after(&self) -> u8 {
        {
            let offset = ::binparse::Len {
                byte: 0usize,
                bit: 0usize,
            }
                + ({
                    {
                        let start = 0usize;
                        let byte = match self
                            .data
                            .get(start..)
                            .and_then(|rest| rest.iter().position(|&b| b == 0u8))
                        {
                            Some(pos) => pos.saturating_add(1),
                            None => {
                                self.data.len().saturating_add(1).saturating_sub(start)
                            }
                        };
                        ::binparse::Len { byte, bit: 0 }
                    }
                });
            debug_assert!(offset.bit == 0, "primitive requires byte alignment");
            self.data[offset.byte]
        }
    }
    pub fn after_end_offset(&self) -> binparse::Len {
        ({
            ::binparse::Len {
                byte: 0usize,
                bit: 0usize,
            }
                + ({
                    {
                        let start = 0usize;
                        let byte = match self
                            .data
                            .get(start..)
                            .and_then(|rest| rest.iter().position(|&b| b == 0u8))
                        {
                            Some(pos) => pos.saturating_add(1),
                            None => {
                                self.data.len().saturating_add(1).saturating_sub(start)
                            }
                        };
                        ::binparse::Len { byte, bit: 0 }
                    }
                })
        })
            + ::binparse::Len {
                byte: 1usize,
                bit: 0usize,
            }
    }
    pub fn after_start_offset(&self) -> binparse::Len {
        self.name_end_offset()
    }
    pub fn after_bit_range(&self) -> ::core::ops::Range<usize> {
        self.after_start_offset().bits()..self.after_end_offset().bits()
    }
    #[allow(dead_code)]
    fn after_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.after_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn after_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for CStr<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        CStr::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        CStr::handoff(self)
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
pub struct StructArray_items_Iterator<'a> {
    idx: usize,
    count: usize,
    data: &'a [u8],
}
impl<'a> ::std::iter::Iterator for StructArray_items_Iterator<'a> {
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
        me.count_fatal_check()?;
        me.count_recoverable_check()?;
        me.items_fatal_check()?;
        me.items_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.count_present_node());
        }
        {
            let me = self;
            children.push(me.items_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "StructArray",
                "StructArray",
                0usize..self.items_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.count_fatal_check() {
                Err(error) => {
                    let start = me.count_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "count",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.count_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.count_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "count",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.count_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.items_fatal_check() {
                Err(error) => {
                    let start = me.items_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "items",
                                    "[Inner]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.items_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.items_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "items",
                                            "[Inner]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.items_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "StructArray",
                "StructArray",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn count_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.count_bit_range();
        ::binparse::FieldNode::new(
            "count",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.count())),
        )
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
    pub fn count_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn count_bit_range(&self) -> ::core::ops::Range<usize> {
        self.count_start_offset().bits()..self.count_end_offset().bits()
    }
    #[allow(dead_code)]
    fn count_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.count_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn count_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn items_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.items_bit_range();
        {
            let mut elem_nodes = ::std::vec::Vec::new();
            if let Ok(iter) = self.items() {
                let mut start = bit_range.start;
                for (i, elem) in iter.enumerate() {
                    match elem {
                        Ok(value) => {
                            let end = start.saturating_add(value.a_end_offset().bits());
                            elem_nodes
                                .push(
                                    value.field_tree().renamed(i.to_string()).shifted(start),
                                );
                            start = end;
                        }
                        Err(error) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                            i.to_string(),
                                            "Inner",
                                            start..start,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                    }
                }
            }
            ::binparse::FieldNode::new(
                    "items",
                    "[Inner]",
                    bit_range.clone(),
                    ::binparse::Value::Array,
                )
                .with_children(elem_nodes)
        }
    }
    #[allow(clippy::identity_op)]
    pub fn items(&self) -> ::binparse::ParseResult<StructArray_items_Iterator<'a>> {
        Ok(StructArray_items_Iterator {
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
                    byte: 1usize,
                    bit: 0usize,
                } * (self.count() as usize)
            })
    }
    pub fn items_start_offset(&self) -> binparse::Len {
        self.count_end_offset()
    }
    pub fn items_bit_range(&self) -> ::core::ops::Range<usize> {
        self.items_start_offset().bits()..self.items_end_offset().bits()
    }
    #[allow(dead_code)]
    fn items_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.items_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn items_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for StructArray<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        StructArray::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        StructArray::handoff(self)
    }
}
pub struct Inner<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Inner<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.a_fatal_check()?;
        me.a_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.a_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Inner",
                "Inner",
                0usize..self.a_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.a_fatal_check() {
                Err(error) => {
                    let start = me.a_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "a",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.a_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.a_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "a",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.a_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Inner",
                "Inner",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn a_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.a_bit_range();
        ::binparse::FieldNode::new(
            "a",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.a())),
        )
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
    pub fn a_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn a_bit_range(&self) -> ::core::ops::Range<usize> {
        self.a_start_offset().bits()..self.a_end_offset().bits()
    }
    #[allow(dead_code)]
    fn a_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.a_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn a_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Inner<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Inner::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Inner::handoff(self)
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
pub struct BitArray_nibbles_Iterator<'a> {
    idx: usize,
    count: usize,
    data: &'a [u8],
    bit_offset: usize,
}
impl<'a> ::std::iter::Iterator for BitArray_nibbles_Iterator<'a> {
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
            (self.data[byte_idx] >> (8 - bit_idx - 4usize)) & mask
        } else {
            let bits_in_first = 8 - bit_idx;
            let bits_in_second = 4usize - bits_in_first;
            let first_mask = (1u8 << bits_in_first) - 1;
            let first_part = self.data[byte_idx] & first_mask;
            let second_part = self.data[byte_idx + 1] >> (8 - bits_in_second);
            (first_part << bits_in_second) | second_part
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
        me.nibbles_fatal_check()?;
        me.nibbles_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.nibbles_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "BitArray",
                "BitArray",
                0usize..self.nibbles_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.nibbles_fatal_check() {
                Err(error) => {
                    let start = me.nibbles_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "nibbles",
                                    "[b<4>]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.nibbles_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.nibbles_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "nibbles",
                                            "[b<4>]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.nibbles_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "BitArray",
                "BitArray",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn nibbles_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.nibbles_bit_range();
        {
            let mut elem_nodes = ::std::vec::Vec::new();
            if let Ok(iter) = self.nibbles() {
                let mut start = bit_range.start;
                for (i, elem) in iter.enumerate() {
                    let end = start.saturating_add(4usize);
                    match elem {
                        Ok(value) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                        i.to_string(),
                                        "b<4>",
                                        start..end,
                                        ::binparse::Value::UInt(u128::from(value)),
                                    ),
                                )
                        }
                        Err(error) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                            i.to_string(),
                                            "b<4>",
                                            start..start,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                    }
                    start = end;
                }
            }
            ::binparse::FieldNode::new(
                    "nibbles",
                    "[b<4>]",
                    bit_range.clone(),
                    ::binparse::Value::Array,
                )
                .with_children(elem_nodes)
        }
    }
    #[allow(clippy::identity_op)]
    pub fn nibbles(&self) -> ::binparse::ParseResult<BitArray_nibbles_Iterator<'a>> {
        Ok(BitArray_nibbles_Iterator {
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
    pub fn nibbles_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn nibbles_bit_range(&self) -> ::core::ops::Range<usize> {
        self.nibbles_start_offset().bits()..self.nibbles_end_offset().bits()
    }
    #[allow(dead_code)]
    fn nibbles_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.nibbles_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn nibbles_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for BitArray<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        BitArray::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        BitArray::handoff(self)
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
        me.flags_fatal_check()?;
        me.flags_recoverable_check()?;
        me.fragment_offset_fatal_check()?;
        me.fragment_offset_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.flags_present_node());
        }
        {
            let me = self;
            children.push(me.fragment_offset_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "WithConcat",
                "WithConcat",
                0usize..self.fragment_offset_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.flags_fatal_check() {
                Err(error) => {
                    let start = me.flags_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "flags",
                                    "b<3>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.flags_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.flags_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "flags",
                                            "b<3>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.flags_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.fragment_offset_fatal_check() {
                Err(error) => {
                    let start = me.fragment_offset_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "fragment_offset",
                                    "concat",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.fragment_offset_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.fragment_offset_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "fragment_offset",
                                            "concat",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.fragment_offset_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "WithConcat",
                "WithConcat",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn flags_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.flags_bit_range();
        ::binparse::FieldNode::new(
            "flags",
            "b<3>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.flags())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn flags(&self) -> u8 {
        (self.data[0usize] >> 5usize) & 7u8
    }
    pub fn flags_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 0usize,
            bit: 3usize,
        }
    }
    pub fn flags_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn flags_bit_range(&self) -> ::core::ops::Range<usize> {
        self.flags_start_offset().bits()..self.flags_end_offset().bits()
    }
    #[allow(dead_code)]
    fn flags_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.flags_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn flags_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn fragment_offset_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.fragment_offset_bit_range();
        {
            let mut item_nodes = ::std::vec::Vec::new();
            {
                let bit_range = 3usize..8usize;
                item_nodes
                    .push(
                        ::binparse::FieldNode::new(
                            "fragment_offset_0",
                            "b<5>",
                            bit_range.clone(),
                            ::binparse::Value::UInt(u128::from(self.fragment_offset_0())),
                        ),
                    );
            }
            {
                let bit_range = 8usize..16usize;
                item_nodes
                    .push(
                        ::binparse::FieldNode::new(
                            "fragment_offset_1",
                            "u8",
                            bit_range.clone(),
                            ::binparse::Value::UInt(u128::from(self.fragment_offset_1())),
                        ),
                    );
            }
            ::binparse::FieldNode::new(
                    "fragment_offset",
                    "concat",
                    bit_range.clone(),
                    ::binparse::Value::Struct,
                )
                .with_children(item_nodes)
        }
    }
    #[allow(clippy::identity_op)]
    pub fn fragment_offset_0(&self) -> u8 {
        (self.data[0usize] >> 0usize) & 31u8
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
    pub fn fragment_offset_start_offset(&self) -> binparse::Len {
        self.flags_end_offset()
    }
    pub fn fragment_offset_bit_range(&self) -> ::core::ops::Range<usize> {
        self
            .fragment_offset_start_offset()
            .bits()..self.fragment_offset_end_offset().bits()
    }
    #[allow(dead_code)]
    fn fragment_offset_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.fragment_offset_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn fragment_offset_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for WithConcat<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        WithConcat::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        WithConcat::handoff(self)
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
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.id_fatal_check()?;
        me.id_recoverable_check()?;
        let len = me.id_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.id_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Packet_payload_Echo",
                "Packet_payload_Echo",
                0usize..self.id_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.id_fatal_check() {
                Err(error) => {
                    let start = me.id_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "id",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.id_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.id_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "id",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.id_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Packet_payload_Echo",
                "Packet_payload_Echo",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn id_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.id_bit_range();
        ::binparse::FieldNode::new(
            "id",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.id())),
        )
    }
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
    pub fn id_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn id_bit_range(&self) -> ::core::ops::Range<usize> {
        self.id_start_offset().bits()..self.id_end_offset().bits()
    }
    #[allow(dead_code)]
    fn id_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.id_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn id_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Packet_payload_Echo<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Packet_payload_Echo::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Packet_payload_Echo::handoff(self)
    }
}
#[allow(non_camel_case_types)]
pub struct Packet_payload_Unknown<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Packet_payload_Unknown<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        Ok((Self { data }, data))
    }
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let children = ::std::vec::Vec::new();
        let mut root = ::binparse::FieldNode::new(
                "Packet_payload_Unknown",
                "Packet_payload_Unknown",
                0usize..0usize,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let _ = Self { data };
        let children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut root = ::binparse::FieldNode::new(
                "Packet_payload_Unknown",
                "Packet_payload_Unknown",
                0usize..0usize,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
}
impl<'a> ::binparse::Dissect<'a> for Packet_payload_Unknown<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Packet_payload_Unknown::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Packet_payload_Unknown::handoff(self)
    }
}
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
        me.ty_fatal_check()?;
        me.ty_recoverable_check()?;
        me.payload_fatal_check()?;
        me.payload_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.ty_present_node());
        }
        {
            let me = self;
            children.push(me.payload_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Packet",
                "Packet",
                0usize..self.payload_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.ty_fatal_check() {
                Err(error) => {
                    let start = me.ty_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "ty",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.ty_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.ty_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "ty",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.ty_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.payload_fatal_check() {
                Err(error) => {
                    let start = me.payload_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "payload",
                                    "union",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.payload_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.payload_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "payload",
                                            "union",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.payload_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Packet",
                "Packet",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn ty_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.ty_bit_range();
        ::binparse::FieldNode::new(
            "ty",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.ty())),
        )
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
    pub fn ty_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn ty_bit_range(&self) -> ::core::ops::Range<usize> {
        self.ty_start_offset().bits()..self.ty_end_offset().bits()
    }
    #[allow(dead_code)]
    fn ty_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.ty_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn ty_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn payload_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.payload_bit_range();
        match self.payload() {
            Ok(Packet_payload::Echo(value)) => {
                let inner = value.field_tree().renamed("Echo").shifted(bit_range.start);
                ::binparse::FieldNode::new(
                        "payload",
                        "union",
                        bit_range.clone(),
                        ::binparse::Value::UnionVariant("Echo"),
                    )
                    .with_children(::std::vec![inner])
            }
            Ok(Packet_payload::Unknown(value)) => {
                let inner = value
                    .field_tree()
                    .renamed("Unknown")
                    .shifted(bit_range.start);
                ::binparse::FieldNode::new(
                        "payload",
                        "union",
                        bit_range.clone(),
                        ::binparse::Value::UnionVariant("Unknown"),
                    )
                    .with_children(::std::vec![inner])
            }
            Err(error) => {
                ::binparse::FieldNode::new(
                        "payload",
                        "union",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Error(error))
            }
        }
    }
    fn payload_union_check(&self) -> Result<(), binparse::ParseError> {
        match self.ty() as usize {
            1 => {
                Packet_payload_Echo::parse(&self.data[(1usize).min(self.data.len())..])?;
            }
            _ => {
                Packet_payload_Unknown::parse(
                    &self.data[(1usize).min(self.data.len())..],
                )?;
            }
        }
        Ok(())
    }
    #[allow(clippy::identity_op)]
    pub fn payload(&self) -> ::binparse::ParseResult<Packet_payload<'a>> {
        match self.ty() as usize {
            1 => {
                Packet_payload_Echo::parse(&self.data[1usize..])
                    .map(|(value, _)| Packet_payload::Echo(value))
            }
            _ => {
                Packet_payload_Unknown::parse(&self.data[1usize..])
                    .map(|(value, _)| Packet_payload::Unknown(value))
            }
        }
    }
    pub fn payload_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
            + ({
                match self.ty() as usize {
                    1 => {
                        ::binparse::Len {
                            byte: 2usize,
                            bit: 0usize,
                        }
                    }
                    _ => {
                        ::binparse::Len {
                            byte: 0usize,
                            bit: 0usize,
                        }
                    }
                }
            })
    }
    pub fn payload_start_offset(&self) -> binparse::Len {
        self.ty_end_offset()
    }
    pub fn payload_bit_range(&self) -> ::core::ops::Range<usize> {
        self.payload_start_offset().bits()..self.payload_end_offset().bits()
    }
    #[allow(dead_code)]
    fn payload_fatal_check(&self) -> Result<(), binparse::ParseError> {
        self.payload_union_check()?;
        {
            let len = self.payload_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn payload_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Packet<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Packet::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Packet::handoff(self)
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
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.id_fatal_check()?;
        me.id_recoverable_check()?;
        let len = me.id_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.id_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Packet_payload_Echo",
                "Packet_payload_Echo",
                0usize..self.id_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.id_fatal_check() {
                Err(error) => {
                    let start = me.id_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "id",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.id_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.id_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "id",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.id_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Packet_payload_Echo",
                "Packet_payload_Echo",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn id_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.id_bit_range();
        ::binparse::FieldNode::new(
            "id",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.id())),
        )
    }
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
    pub fn id_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn id_bit_range(&self) -> ::core::ops::Range<usize> {
        self.id_start_offset().bits()..self.id_end_offset().bits()
    }
    #[allow(dead_code)]
    fn id_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.id_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn id_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Packet_payload_Echo<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Packet_payload_Echo::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Packet_payload_Echo::handoff(self)
    }
}
#[allow(non_camel_case_types)]
pub struct Packet_payload_Unknown<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Packet_payload_Unknown<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        Ok((Self { data }, data))
    }
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let children = ::std::vec::Vec::new();
        let mut root = ::binparse::FieldNode::new(
                "Packet_payload_Unknown",
                "Packet_payload_Unknown",
                0usize..0usize,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let _ = Self { data };
        let children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut root = ::binparse::FieldNode::new(
                "Packet_payload_Unknown",
                "Packet_payload_Unknown",
                0usize..0usize,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
}
impl<'a> ::binparse::Dissect<'a> for Packet_payload_Unknown<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Packet_payload_Unknown::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Packet_payload_Unknown::handoff(self)
    }
}
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
        me.ty_fatal_check()?;
        me.ty_recoverable_check()?;
        me.code_fatal_check()?;
        me.code_recoverable_check()?;
        me.payload_fatal_check()?;
        me.payload_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.ty_present_node());
        }
        {
            let me = self;
            children.push(me.code_present_node());
        }
        {
            let me = self;
            children.push(me.payload_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Packet",
                "Packet",
                0usize..self.payload_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.ty_fatal_check() {
                Err(error) => {
                    let start = me.ty_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "ty",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.ty_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.ty_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "ty",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.ty_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.code_fatal_check() {
                Err(error) => {
                    let start = me.code_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "code",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.code_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.code_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "code",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.code_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.payload_fatal_check() {
                Err(error) => {
                    let start = me.payload_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "payload",
                                    "union",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.payload_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.payload_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "payload",
                                            "union",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.payload_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Packet",
                "Packet",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn ty_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.ty_bit_range();
        ::binparse::FieldNode::new(
            "ty",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.ty())),
        )
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
    pub fn ty_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn ty_bit_range(&self) -> ::core::ops::Range<usize> {
        self.ty_start_offset().bits()..self.ty_end_offset().bits()
    }
    #[allow(dead_code)]
    fn ty_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.ty_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn ty_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn code_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.code_bit_range();
        ::binparse::FieldNode::new(
            "code",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.code())),
        )
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
    pub fn code_start_offset(&self) -> binparse::Len {
        self.ty_end_offset()
    }
    pub fn code_bit_range(&self) -> ::core::ops::Range<usize> {
        self.code_start_offset().bits()..self.code_end_offset().bits()
    }
    #[allow(dead_code)]
    fn code_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.code_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn code_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn payload_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.payload_bit_range();
        match self.payload() {
            Ok(Packet_payload::Echo(value)) => {
                let inner = value.field_tree().renamed("Echo").shifted(bit_range.start);
                ::binparse::FieldNode::new(
                        "payload",
                        "union",
                        bit_range.clone(),
                        ::binparse::Value::UnionVariant("Echo"),
                    )
                    .with_children(::std::vec![inner])
            }
            Ok(Packet_payload::Unknown(value)) => {
                let inner = value
                    .field_tree()
                    .renamed("Unknown")
                    .shifted(bit_range.start);
                ::binparse::FieldNode::new(
                        "payload",
                        "union",
                        bit_range.clone(),
                        ::binparse::Value::UnionVariant("Unknown"),
                    )
                    .with_children(::std::vec![inner])
            }
            Err(error) => {
                ::binparse::FieldNode::new(
                        "payload",
                        "union",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Error(error))
            }
        }
    }
    fn payload_union_check(&self) -> Result<(), binparse::ParseError> {
        match (self.ty() as usize, self.code() as usize) {
            (0, 0) | (0, 8) => {
                Packet_payload_Echo::parse(&self.data[(2usize).min(self.data.len())..])?;
            }
            _ => {
                Packet_payload_Unknown::parse(
                    &self.data[(2usize).min(self.data.len())..],
                )?;
            }
        }
        Ok(())
    }
    #[allow(clippy::identity_op)]
    pub fn payload(&self) -> ::binparse::ParseResult<Packet_payload<'a>> {
        match (self.ty() as usize, self.code() as usize) {
            (0, 0) | (0, 8) => {
                Packet_payload_Echo::parse(&self.data[2usize..])
                    .map(|(value, _)| Packet_payload::Echo(value))
            }
            _ => {
                Packet_payload_Unknown::parse(&self.data[2usize..])
                    .map(|(value, _)| Packet_payload::Unknown(value))
            }
        }
    }
    pub fn payload_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
            + ({
                match (self.ty() as usize, self.code() as usize) {
                    (0, 0) | (0, 8) => {
                        ::binparse::Len {
                            byte: 2usize,
                            bit: 0usize,
                        }
                    }
                    _ => {
                        ::binparse::Len {
                            byte: 0usize,
                            bit: 0usize,
                        }
                    }
                }
            })
    }
    pub fn payload_start_offset(&self) -> binparse::Len {
        self.code_end_offset()
    }
    pub fn payload_bit_range(&self) -> ::core::ops::Range<usize> {
        self.payload_start_offset().bits()..self.payload_end_offset().bits()
    }
    #[allow(dead_code)]
    fn payload_fatal_check(&self) -> Result<(), binparse::ParseError> {
        self.payload_union_check()?;
        {
            let len = self.payload_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn payload_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Packet<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Packet::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Packet::handoff(self)
    }
}

"#,
    );
}

#[test]
fn golden_union_error_variant() {
    assert_generated_eq(
        r#"error {
            UNKNOWN_TYPE { ty: u8 },
        }

        struct Packet {
            ty: u8,
            payload: union(ty) {
                1 => Echo { id: u16 },
                _ => @error(UNKNOWN_TYPE { ty: ty }),
            },
        }"#,
        r#"
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Parse(::binparse::ParseError),
    UNKNOWN_TYPE { ty: u8 },
}
impl Error {
    #[allow(dead_code)]
    fn variant_name(&self) -> &'static str {
        match self {
            Error::Parse(_) => "Parse",
            Error::UNKNOWN_TYPE { .. } => "UNKNOWN_TYPE",
        }
    }
}
#[allow(non_camel_case_types)]
pub struct Packet_payload_Echo<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Packet_payload_Echo<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.id_fatal_check()?;
        me.id_recoverable_check()?;
        let len = me.id_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.id_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Packet_payload_Echo",
                "Packet_payload_Echo",
                0usize..self.id_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.id_fatal_check() {
                Err(error) => {
                    let start = me.id_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "id",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.id_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.id_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "id",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.id_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Packet_payload_Echo",
                "Packet_payload_Echo",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn id_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.id_bit_range();
        ::binparse::FieldNode::new(
            "id",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.id())),
        )
    }
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
    pub fn id_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn id_bit_range(&self) -> ::core::ops::Range<usize> {
        self.id_start_offset().bits()..self.id_end_offset().bits()
    }
    #[allow(dead_code)]
    fn id_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.id_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn id_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Packet_payload_Echo<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Packet_payload_Echo::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Packet_payload_Echo::handoff(self)
    }
}
#[allow(non_camel_case_types)]
pub enum Packet_payload<'a> {
    Echo(Packet_payload_Echo<'a>),
}
pub struct Packet<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Packet<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.ty_fatal_check()?;
        me.ty_recoverable_check()?;
        me.payload_fatal_check()?;
        me.payload_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.ty_present_node());
        }
        {
            let me = self;
            children.push(me.payload_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Packet",
                "Packet",
                0usize..self.payload_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.ty_fatal_check() {
                Err(error) => {
                    let start = me.ty_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "ty",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.ty_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.ty_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "ty",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.ty_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.payload_fatal_check() {
                Err(error) => {
                    let start = me.payload_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "payload",
                                    "union",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.payload_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.payload_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "payload",
                                            "union",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.payload_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Packet",
                "Packet",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn ty_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.ty_bit_range();
        ::binparse::FieldNode::new(
            "ty",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.ty())),
        )
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
    pub fn ty_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn ty_bit_range(&self) -> ::core::ops::Range<usize> {
        self.ty_start_offset().bits()..self.ty_end_offset().bits()
    }
    #[allow(dead_code)]
    fn ty_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.ty_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn ty_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn payload_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.payload_bit_range();
        match self.payload() {
            Ok(Packet_payload::Echo(value)) => {
                let inner = value.field_tree().renamed("Echo").shifted(bit_range.start);
                ::binparse::FieldNode::new(
                        "payload",
                        "union",
                        bit_range.clone(),
                        ::binparse::Value::UnionVariant("Echo"),
                    )
                    .with_children(::std::vec![inner])
            }
            Err(Error::Parse(error)) => {
                ::binparse::FieldNode::new(
                        "payload",
                        "union",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Error(error))
            }
            Err(error) => {
                ::binparse::FieldNode::new(
                        "payload",
                        "union",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Failed(error.variant_name()))
            }
        }
    }
    fn payload_union_check(&self) -> Result<(), binparse::ParseError> {
        match self.ty() as usize {
            1 => {
                Packet_payload_Echo::parse(&self.data[(1usize).min(self.data.len())..])?;
            }
            _ => {}
        }
        Ok(())
    }
    #[allow(clippy::identity_op)]
    pub fn payload(&self) -> Result<Packet_payload<'a>, Error> {
        match self.ty() as usize {
            1 => {
                Packet_payload_Echo::parse(&self.data[1usize..])
                    .map(|(value, _)| Packet_payload::Echo(value))
                    .map_err(Error::Parse)
            }
            _ => {
                Err(Error::UNKNOWN_TYPE {
                    ty: (self.ty() as usize) as u8,
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
                match self.ty() as usize {
                    1 => {
                        ::binparse::Len {
                            byte: 2usize,
                            bit: 0usize,
                        }
                    }
                    _ => ::binparse::Len::ZERO,
                }
            })
    }
    pub fn payload_start_offset(&self) -> binparse::Len {
        self.ty_end_offset()
    }
    pub fn payload_bit_range(&self) -> ::core::ops::Range<usize> {
        self.payload_start_offset().bits()..self.payload_end_offset().bits()
    }
    #[allow(dead_code)]
    fn payload_fatal_check(&self) -> Result<(), binparse::ParseError> {
        self.payload_union_check()?;
        {
            let len = self.payload_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn payload_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Packet<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Packet::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Packet::handoff(self)
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
        me.header_fatal_check()?;
        me.header_recoverable_check()?;
        me.mixed_fatal_check()?;
        me.mixed_recoverable_check()?;
        me.data_fatal_check()?;
        me.data_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.header_present_node());
        }
        {
            let me = self;
            children.push(me.mixed_present_node());
        }
        {
            let me = self;
            children.push(me.data_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "LittlePacket",
                "LittlePacket",
                0usize..self.data_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.header_fatal_check() {
                Err(error) => {
                    let start = me.header_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "header",
                                    "u32",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.header_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.header_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "header",
                                            "u32",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.header_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.mixed_fatal_check() {
                Err(error) => {
                    let start = me.mixed_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "mixed",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.mixed_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.mixed_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "mixed",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.mixed_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.data_fatal_check() {
                Err(error) => {
                    let start = me.data_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "data",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.data_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.data_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "data",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.data_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "LittlePacket",
                "LittlePacket",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn header_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.header_bit_range();
        ::binparse::FieldNode::new(
            "header",
            "u32",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.header())),
        )
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
    pub fn header_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn header_bit_range(&self) -> ::core::ops::Range<usize> {
        self.header_start_offset().bits()..self.header_end_offset().bits()
    }
    #[allow(dead_code)]
    fn header_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.header_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn header_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn mixed_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.mixed_bit_range();
        ::binparse::FieldNode::new(
            "mixed",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.mixed())),
        )
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
    pub fn mixed_start_offset(&self) -> binparse::Len {
        self.header_end_offset()
    }
    pub fn mixed_bit_range(&self) -> ::core::ops::Range<usize> {
        self.mixed_start_offset().bits()..self.mixed_end_offset().bits()
    }
    #[allow(dead_code)]
    fn mixed_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.mixed_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn mixed_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn data_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.data_bit_range();
        ::binparse::FieldNode::new(
            "data",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.data())),
        )
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
    pub fn data_start_offset(&self) -> binparse::Len {
        self.mixed_end_offset()
    }
    pub fn data_bit_range(&self) -> ::core::ops::Range<usize> {
        self.data_start_offset().bits()..self.data_end_offset().bits()
    }
    #[allow(dead_code)]
    fn data_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.data_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn data_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for LittlePacket<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        LittlePacket::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        LittlePacket::handoff(self)
    }
}

"#,
    );
}

#[test]
fn golden_signed_primitives() {
    assert_generated_eq(
        "@endian(little) struct SignedPrim { a: i8, b: i16, @endian(big) c: i32 }",
        r#"
pub struct SignedPrim<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> SignedPrim<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.a_fatal_check()?;
        me.a_recoverable_check()?;
        me.b_fatal_check()?;
        me.b_recoverable_check()?;
        me.c_fatal_check()?;
        me.c_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.a_present_node());
        }
        {
            let me = self;
            children.push(me.b_present_node());
        }
        {
            let me = self;
            children.push(me.c_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "SignedPrim",
                "SignedPrim",
                0usize..self.c_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.a_fatal_check() {
                Err(error) => {
                    let start = me.a_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "a",
                                    "i8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.a_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.a_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "a",
                                            "i8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.a_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.b_fatal_check() {
                Err(error) => {
                    let start = me.b_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "b",
                                    "i16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.b_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.b_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "b",
                                            "i16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.b_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.c_fatal_check() {
                Err(error) => {
                    let start = me.c_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "c",
                                    "i32",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.c_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.c_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "c",
                                            "i32",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.c_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "SignedPrim",
                "SignedPrim",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn a_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.a_bit_range();
        ::binparse::FieldNode::new(
            "a",
            "i8",
            bit_range.clone(),
            ::binparse::Value::Int(i128::from(self.a())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn a(&self) -> i8 {
        self.data[0usize] as i8
    }
    pub fn a_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
    }
    pub fn a_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn a_bit_range(&self) -> ::core::ops::Range<usize> {
        self.a_start_offset().bits()..self.a_end_offset().bits()
    }
    #[allow(dead_code)]
    fn a_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.a_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn a_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn b_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.b_bit_range();
        ::binparse::FieldNode::new(
            "b",
            "i16",
            bit_range.clone(),
            ::binparse::Value::Int(i128::from(self.b())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn b(&self) -> i16 {
        i16::from_le_bytes(self.data[1usize..3usize].try_into().unwrap())
    }
    pub fn b_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 3usize,
            bit: 0usize,
        }
    }
    pub fn b_start_offset(&self) -> binparse::Len {
        self.a_end_offset()
    }
    pub fn b_bit_range(&self) -> ::core::ops::Range<usize> {
        self.b_start_offset().bits()..self.b_end_offset().bits()
    }
    #[allow(dead_code)]
    fn b_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.b_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn b_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn c_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.c_bit_range();
        ::binparse::FieldNode::new(
            "c",
            "i32",
            bit_range.clone(),
            ::binparse::Value::Int(i128::from(self.c())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn c(&self) -> i32 {
        i32::from_be_bytes(self.data[3usize..7usize].try_into().unwrap())
    }
    pub fn c_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 7usize,
            bit: 0usize,
        }
    }
    pub fn c_start_offset(&self) -> binparse::Len {
        self.b_end_offset()
    }
    pub fn c_bit_range(&self) -> ::core::ops::Range<usize> {
        self.c_start_offset().bits()..self.c_end_offset().bits()
    }
    #[allow(dead_code)]
    fn c_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.c_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn c_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for SignedPrim<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        SignedPrim::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        SignedPrim::handoff(self)
    }
}

"#,
    );
}

#[test]
fn golden_lsb_bit_order() {
    assert_generated_eq(
        "@bit_order(lsb) struct LsbBits { low: b<3>, high: b<5> }",
        r#"
pub struct LsbBits<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> LsbBits<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.low_fatal_check()?;
        me.low_recoverable_check()?;
        me.high_fatal_check()?;
        me.high_recoverable_check()?;
        let len = me.high_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.low_present_node());
        }
        {
            let me = self;
            children.push(me.high_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "LsbBits",
                "LsbBits",
                0usize..self.high_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.low_fatal_check() {
                Err(error) => {
                    let start = me.low_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "low",
                                    "b<3>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.low_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.low_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "low",
                                            "b<3>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.low_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.high_fatal_check() {
                Err(error) => {
                    let start = me.high_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "high",
                                    "b<5>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.high_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.high_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "high",
                                            "b<5>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.high_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "LsbBits",
                "LsbBits",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn low_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.low_bit_range();
        ::binparse::FieldNode::new(
            "low",
            "b<3>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.low())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn low(&self) -> u8 {
        (self.data[0usize] >> 0usize) & 7u8
    }
    pub fn low_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 0usize,
            bit: 3usize,
        }
    }
    pub fn low_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn low_bit_range(&self) -> ::core::ops::Range<usize> {
        self.low_start_offset().bits()..self.low_end_offset().bits()
    }
    #[allow(dead_code)]
    fn low_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.low_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn low_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn high_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.high_bit_range();
        ::binparse::FieldNode::new(
            "high",
            "b<5>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.high())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn high(&self) -> u8 {
        (self.data[0usize] >> 3usize) & 31u8
    }
    pub fn high_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
    }
    pub fn high_start_offset(&self) -> binparse::Len {
        self.low_end_offset()
    }
    pub fn high_bit_range(&self) -> ::core::ops::Range<usize> {
        self.high_start_offset().bits()..self.high_end_offset().bits()
    }
    #[allow(dead_code)]
    fn high_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.high_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn high_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for LsbBits<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        LsbBits::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        LsbBits::handoff(self)
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
        me.prefix_fatal_check()?;
        me.prefix_recoverable_check()?;
        me.value_fatal_check()?;
        me.value_recoverable_check()?;
        me.suffix_fatal_check()?;
        me.suffix_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.prefix_present_node());
        }
        {
            let me = self;
            children.push(me.value_present_node());
        }
        {
            let me = self;
            children.push(me.suffix_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "WithFixedHook",
                "WithFixedHook",
                0usize..self.suffix_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.prefix_fatal_check() {
                Err(error) => {
                    let start = me.prefix_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "prefix",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.prefix_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.prefix_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "prefix",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.prefix_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.value_fatal_check() {
                Err(error) => {
                    let start = me.value_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "value",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.value_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.value_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "value",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.value_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.suffix_fatal_check() {
                Err(error) => {
                    let start = me.suffix_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "suffix",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.suffix_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.suffix_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "suffix",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.suffix_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "WithFixedHook",
                "WithFixedHook",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn prefix_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.prefix_bit_range();
        ::binparse::FieldNode::new(
            "prefix",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.prefix())),
        )
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
    pub fn prefix_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn prefix_bit_range(&self) -> ::core::ops::Range<usize> {
        self.prefix_start_offset().bits()..self.prefix_end_offset().bits()
    }
    #[allow(dead_code)]
    fn prefix_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.prefix_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn prefix_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn value_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.value_bit_range();
        match self.value() {
            Ok(value) => {
                ::binparse::FieldNode::new(
                    "value",
                    "u32",
                    bit_range.clone(),
                    ::binparse::Value::UInt(u128::from(value)),
                )
            }
            Err(error) => {
                ::binparse::FieldNode::new(
                        "value",
                        "u32",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Error(error))
            }
        }
    }
    #[allow(clippy::identity_op)]
    pub fn value(&self) -> ::binparse::ParseResult<u32> {
        let start = 1usize;
        double_it(
            u16::from_be_bytes(self.data[1usize..3usize].try_into().unwrap()),
            ::binparse::HookContext {
                field: "WithFixedHook.value",
                offset: start,
                enclosing: self.data,
            },
        )
    }
    pub fn value_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 3usize,
            bit: 0usize,
        }
    }
    pub fn value_start_offset(&self) -> binparse::Len {
        self.prefix_end_offset()
    }
    pub fn value_bit_range(&self) -> ::core::ops::Range<usize> {
        self.value_start_offset().bits()..self.value_end_offset().bits()
    }
    #[allow(dead_code)]
    fn value_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.value_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn value_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        self.value()?;
        Ok(())
    }
    #[allow(dead_code)]
    fn suffix_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.suffix_bit_range();
        ::binparse::FieldNode::new(
            "suffix",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.suffix())),
        )
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
    pub fn suffix_start_offset(&self) -> binparse::Len {
        self.value_end_offset()
    }
    pub fn suffix_bit_range(&self) -> ::core::ops::Range<usize> {
        self.suffix_start_offset().bits()..self.suffix_end_offset().bits()
    }
    #[allow(dead_code)]
    fn suffix_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.suffix_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn suffix_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for WithFixedHook<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        WithFixedHook::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        WithFixedHook::handoff(self)
    }
}

"#,
    );
}

#[test]
fn golden_len_bounded_hook() {
    assert_generated_eq(
        r#"struct WithLenHook {
            len: u8,
            @len(len) @hook(read_leb128, u64) value: [u8],
            after: u8,
        }"#,
        r#"
impl<'a> ::binparse::Dissect<'a> for WithLenHook<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        WithLenHook::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        WithLenHook::handoff(self)
    }
}

impl<'a> WithLenHook<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.len_fatal_check()?;
        me.len_recoverable_check()?;
        me.value_fatal_check()?;
        me.value_recoverable_check()?;
        me.after_fatal_check()?;
        me.after_recoverable_check()?;
        let len = me.after_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.len_present_node());
        }
        {
            let me = self;
            children.push(me.value_present_node());
        }
        {
            let me = self;
            children.push(me.after_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "WithLenHook",
                "WithLenHook",
                0usize..self.after_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.len_fatal_check() {
                Err(error) => {
                    let start = me.len_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "len",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.len_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.len_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "len",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.len_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.value_fatal_check() {
                Err(error) => {
                    let start = me.value_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "value",
                                    "[u8]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.value_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.value_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "value",
                                            "[u8]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.value_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.after_fatal_check() {
                Err(error) => {
                    let start = me.after_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "after",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.after_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.after_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "after",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.after_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "WithLenHook",
                "WithLenHook",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn len_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.len_bit_range();
        ::binparse::FieldNode::new(
            "len",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.len())),
        )
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
    pub fn len_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn len_bit_range(&self) -> ::core::ops::Range<usize> {
        self.len_start_offset().bits()..self.len_end_offset().bits()
    }
    #[allow(dead_code)]
    fn len_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.len_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn len_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn value_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.value_bit_range();
        match self.value_raw() {
            Ok((value, consumed)) => {
                let consumed_end = bit_range
                    .start
                    .saturating_add(consumed.saturating_mul(8))
                    .min(bit_range.end);
                let mut node = ::binparse::FieldNode::new(
                    "value",
                    "u64",
                    bit_range.clone(),
                    ::binparse::Value::UInt(u128::from(value)),
                );
                if let Ok(rest) = self.value_rest() && !rest.is_empty() {
                    node.children
                        .push(
                            ::binparse::FieldNode::new(
                                "rest",
                                "[u8]",
                                consumed_end..bit_range.end,
                                ::binparse::Value::Bytes(rest),
                            ),
                        );
                }
                node
            }
            Err(error) => {
                ::binparse::FieldNode::new(
                        "value",
                        "u64",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Error(error))
            }
        }
    }
    fn value_raw(&self) -> ::binparse::ParseResult<(u64, usize)> {
        let start = 1usize;
        let window = start.saturating_add(self.len() as usize).min(self.data.len());
        let (value, consumed) = read_leb128(
            &self.data[start.min(window)..window],
            ::binparse::HookContext {
                field: "WithLenHook.value",
                offset: start,
                enclosing: self.data,
            },
        )?;
        let end = start.saturating_add(consumed);
        if end > window {
            return Err(::binparse::ParseError::NotEnoughData {
                expected: end,
                got: window,
            });
        }
        Ok((value, consumed))
    }
    pub fn value_rest(&self) -> ::binparse::ParseResult<&'a [u8]> {
        let start = 1usize;
        let window = start.saturating_add(self.len() as usize).min(self.data.len());
        let (_, consumed) = self.value_raw()?;
        let rest_start = start.saturating_add(consumed).min(window);
        Ok(&self.data[rest_start..window])
    }
    pub fn value(&self) -> ::binparse::ParseResult<u64> {
        self.value_raw().map(|(value, _)| value)
    }
    pub fn value_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
            + ({
                ::binparse::Len {
                    byte: self.len() as usize,
                    bit: 0,
                }
            })
    }
    pub fn value_start_offset(&self) -> binparse::Len {
        self.len_end_offset()
    }
    pub fn value_bit_range(&self) -> ::core::ops::Range<usize> {
        self.value_start_offset().bits()..self.value_end_offset().bits()
    }
    #[allow(dead_code)]
    fn value_fatal_check(&self) -> Result<(), binparse::ParseError> {
        self.value_raw()?;
        {
            let len = self.value_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn value_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn after_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.after_bit_range();
        ::binparse::FieldNode::new(
            "after",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.after())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn after(&self) -> u8 {
        {
            let offset = ::binparse::Len {
                byte: 1usize,
                bit: 0usize,
            }
                + ({
                    ::binparse::Len {
                        byte: self.len() as usize,
                        bit: 0,
                    }
                });
            debug_assert!(offset.bit == 0, "primitive requires byte alignment");
            self.data[offset.byte]
        }
    }
    pub fn after_end_offset(&self) -> binparse::Len {
        ({
            ::binparse::Len {
                byte: 1usize,
                bit: 0usize,
            }
                + ({
                    ::binparse::Len {
                        byte: self.len() as usize,
                        bit: 0,
                    }
                })
        })
            + ::binparse::Len {
                byte: 1usize,
                bit: 0usize,
            }
    }
    pub fn after_start_offset(&self) -> binparse::Len {
        self.value_end_offset()
    }
    pub fn after_bit_range(&self) -> ::core::ops::Range<usize> {
        self.after_start_offset().bits()..self.after_end_offset().bits()
    }
    #[allow(dead_code)]
    fn after_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.after_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn after_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}

pub struct WithLenHook<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
"#,
    );
}

#[test]
fn golden_conditional_hook() {
    assert_generated_eq(
        r#"struct WithCondHook {
            kind: u8,
            if (kind == 1) {
                @hook(double_it, u32) value: u16,
            }
            tail: u8,
        }"#,
        r#"
impl<'a> ::binparse::Dissect<'a> for WithCondHook<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        WithCondHook::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        WithCondHook::handoff(self)
    }
}

impl<'a> WithCondHook<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.kind_fatal_check()?;
        me.kind_recoverable_check()?;
        if me.conditional_0_present() {
            me.value_fatal_check()?;
            me.value_recoverable_check()?;
        }
        me.tail_fatal_check()?;
        me.tail_recoverable_check()?;
        let len = me.tail_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.kind_present_node());
        }
        {
            let me = self;
            if me.conditional_0_present() {
                children.push(me.value_present_node());
            } else {
                children.push(me.value_absent_node());
            }
        }
        {
            let me = self;
            children.push(me.tail_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "WithCondHook",
                "WithCondHook",
                0usize..self.tail_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.kind_fatal_check() {
                Err(error) => {
                    let start = me.kind_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "kind",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.kind_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.kind_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "kind",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.kind_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            if me.conditional_0_present() {
                match me.value_fatal_check() {
                    Err(error) => {
                        let start = me.value_start_offset().bits();
                        children
                            .push(
                                ::binparse::FieldNode::new(
                                        "value",
                                        "u16",
                                        start..start,
                                        ::binparse::Value::Opaque,
                                    )
                                    .with_status(::binparse::Status::Error(error)),
                            );
                        fatal = Some(error);
                    }
                    Ok(()) => {
                        match me.value_recoverable_check() {
                            Err(error) => {
                                let bit_range = me.value_bit_range();
                                children
                                    .push(
                                        ::binparse::FieldNode::new(
                                                "value",
                                                "u16",
                                                bit_range,
                                                ::binparse::Value::Opaque,
                                            )
                                            .with_status(::binparse::Status::Error(error)),
                                    );
                            }
                            Ok(()) => {
                                children.push(me.value_present_node());
                            }
                        }
                    }
                }
            } else {
                children.push(me.value_absent_node());
            }
        }
        if fatal.is_none() {
            match me.tail_fatal_check() {
                Err(error) => {
                    let start = me.tail_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "tail",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.tail_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.tail_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "tail",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.tail_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "WithCondHook",
                "WithCondHook",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn kind_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.kind_bit_range();
        ::binparse::FieldNode::new(
            "kind",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.kind())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn kind(&self) -> u8 {
        self.data[0usize]
    }
    pub fn kind_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
    }
    pub fn kind_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn kind_bit_range(&self) -> ::core::ops::Range<usize> {
        self.kind_start_offset().bits()..self.kind_end_offset().bits()
    }
    #[allow(dead_code)]
    fn kind_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.kind_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn kind_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code, unused_parens)]
    fn conditional_0_present(&self) -> bool {
        ((self.kind() as usize) == (1usize))
    }
    #[allow(dead_code)]
    fn value_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.value_bit_range();
        match self.value_raw() {
            Ok(value) => {
                ::binparse::FieldNode::new(
                    "value",
                    "u32",
                    bit_range.clone(),
                    ::binparse::Value::UInt(u128::from(value)),
                )
            }
            Err(error) => {
                ::binparse::FieldNode::new(
                        "value",
                        "u32",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Error(error))
            }
        }
    }
    #[allow(dead_code)]
    fn value_absent_node(&self) -> ::binparse::FieldNode<'a> {
        let start = self.value_start_offset().bits();
        ::binparse::FieldNode::new(
                "value",
                "u16",
                start..start,
                ::binparse::Value::Absent,
            )
            .hide()
    }
    #[allow(clippy::identity_op)]
    fn value_raw(&self) -> ::binparse::ParseResult<u32> {
        let start = 1usize;
        double_it(
            u16::from_be_bytes(self.data[1usize..3usize].try_into().unwrap()),
            ::binparse::HookContext {
                field: "WithCondHook.value",
                offset: start,
                enclosing: self.data,
            },
        )
    }
    pub fn value(&self) -> Option<::binparse::ParseResult<u32>> {
        if self.conditional_0_present() { Some(self.value_raw()) } else { None }
    }
    pub fn value_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 3usize,
            bit: 0usize,
        }
    }
    pub fn value_start_offset(&self) -> binparse::Len {
        self.kind_end_offset()
    }
    pub fn value_bit_range(&self) -> ::core::ops::Range<usize> {
        self.value_start_offset().bits()..self.value_end_offset().bits()
    }
    #[allow(dead_code)]
    fn value_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.value_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn value_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        self.value_raw()?;
        Ok(())
    }
    fn conditional_0_end_offset(&self) -> binparse::Len {
        if self.conditional_0_present() {
            self.value_end_offset()
        } else {
            self.kind_end_offset()
        }
    }
    #[allow(dead_code)]
    fn tail_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.tail_bit_range();
        ::binparse::FieldNode::new(
            "tail",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.tail())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn tail(&self) -> u8 {
        {
            let offset = self.conditional_0_end_offset();
            debug_assert!(offset.bit == 0, "primitive requires byte alignment");
            self.data[offset.byte]
        }
    }
    pub fn tail_end_offset(&self) -> binparse::Len {
        ({ self.conditional_0_end_offset() })
            + ::binparse::Len {
                byte: 1usize,
                bit: 0usize,
            }
    }
    pub fn tail_start_offset(&self) -> binparse::Len {
        self.conditional_0_end_offset()
    }
    pub fn tail_bit_range(&self) -> ::core::ops::Range<usize> {
        self.tail_start_offset().bits()..self.tail_end_offset().bits()
    }
    #[allow(dead_code)]
    fn tail_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.tail_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn tail_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}

pub struct WithCondHook<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
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
        me.len_fatal_check()?;
        me.len_recoverable_check()?;
        me.name_fatal_check()?;
        me.name_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.len_present_node());
        }
        {
            let me = self;
            children.push(me.name_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "WithVlaHook",
                "WithVlaHook",
                0usize..self.name_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.len_fatal_check() {
                Err(error) => {
                    let start = me.len_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "len",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.len_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.len_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "len",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.len_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.name_fatal_check() {
                Err(error) => {
                    let start = me.name_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "name",
                                    "[u8]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.name_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.name_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "name",
                                            "[u8]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.name_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "WithVlaHook",
                "WithVlaHook",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn len_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.len_bit_range();
        ::binparse::FieldNode::new(
            "len",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.len())),
        )
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
    pub fn len_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn len_bit_range(&self) -> ::core::ops::Range<usize> {
        self.len_start_offset().bits()..self.len_end_offset().bits()
    }
    #[allow(dead_code)]
    fn len_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.len_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn len_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn name_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.name_bit_range();
        match self.name_raw() {
            Ok((_, _)) => {
                ::binparse::FieldNode::new(
                    "name",
                    "String",
                    bit_range.clone(),
                    ::binparse::Value::Opaque,
                )
            }
            Err(error) => {
                ::binparse::FieldNode::new(
                        "name",
                        "String",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Error(error))
            }
        }
    }
    fn name_raw(&self) -> ::binparse::ParseResult<(String, usize)> {
        let start = 1usize;
        let (value, consumed) = parse_cstring(
            &self.data[start..],
            ::binparse::HookContext {
                field: "WithVlaHook.name",
                offset: start,
                enclosing: self.data,
            },
        )?;
        let end = start.saturating_add(consumed);
        if end > self.data.len() {
            return Err(::binparse::ParseError::NotEnoughData {
                expected: end,
                got: self.data.len(),
            });
        }
        Ok((value, consumed))
    }
    pub fn name(&self) -> ::binparse::ParseResult<String> {
        self.name_raw().map(|(value, _)| value)
    }
    pub fn name_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
            + ({
                match self.name_raw() {
                    Ok((_, consumed)) => {
                        binparse::Len {
                            byte: consumed,
                            bit: 0,
                        }
                    }
                    Err(_) => binparse::Len::ZERO,
                }
            })
    }
    pub fn name_start_offset(&self) -> binparse::Len {
        self.len_end_offset()
    }
    pub fn name_bit_range(&self) -> ::core::ops::Range<usize> {
        self.name_start_offset().bits()..self.name_end_offset().bits()
    }
    #[allow(dead_code)]
    fn name_fatal_check(&self) -> Result<(), binparse::ParseError> {
        self.name_raw()?;
        {
            let len = self.name_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn name_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for WithVlaHook<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        WithVlaHook::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        WithVlaHook::handoff(self)
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
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to single-byte integers")
    );
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
fn endian_on_i8_is_rejected() {
    let err = generate_err("struct Foo { @endian(little) a: i8 }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to single-byte integers")
    );
}

#[test]
fn endian_on_single_byte_array_is_rejected() {
    let err = generate_err("struct Foo { @endian(little) xs: [u8; 2] }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to single-byte integers")
    );
}

#[test]
fn endian_on_bitfield_array_is_rejected() {
    let err = generate_err("struct Foo { @endian(little) xs: [b<4>; 2] }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to bitfields")
    );
}

#[test]
fn endian_on_struct_ref_array_is_rejected() {
    let err =
        generate_err("struct Inner { x: u8 } struct Foo { @endian(big) xs: [Inner; 2] }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to struct ref")
    );
}

#[test]
fn bit_order_on_primitive_is_rejected() {
    let err = generate_err("struct Foo { @bit_order(lsb) a: u16 }");
    assert!(
        err.to_string()
            .contains("@bit_order can only be applied to bitfields")
    );
}

#[test]
fn bit_order_on_struct_ref_is_rejected() {
    let err = generate_err("struct Inner { x: u8 } struct Foo { @bit_order(lsb) inner: Inner }");
    assert!(
        err.to_string()
            .contains("@bit_order can only be applied to bitfields")
    );
}

#[test]
fn invalid_bit_order_value_is_rejected() {
    let err = generate_err("struct Foo { @bit_order(big) a: b<4> }");
    assert!(err.to_string().contains("@bit_order argument must be"));
}

#[test]
fn endian_wrong_arg_count_is_rejected() {
    let err = generate_err("struct Foo { @endian(big, little) a: u16 }");
    assert!(err.to_string().contains("requires exactly 1 argument"));
}

#[test]
fn discriminator_on_struct_ref_is_rejected() {
    let err = generate_err(
        "struct Inner { x: u8 } struct Foo { @discriminator inner: Inner, @payload p: [u8; 1] }",
    );
    assert!(
        err.to_string()
            .contains("@discriminator can only be applied to primitive and bitfield fields")
    );
}

#[test]
fn payload_on_primitive_is_rejected() {
    let err = generate_err("struct Foo { @payload x: u16 }");
    assert!(
        err.to_string()
            .contains("@payload can only be applied to byte-array or struct ref fields")
    );
}

#[test]
fn payload_on_non_u8_array_is_rejected() {
    let err = generate_err("struct Foo { @payload x: [u16; 2] }");
    assert!(
        err.to_string()
            .contains("@payload can only be applied to byte-array or struct ref fields")
    );
}

#[test]
fn multiple_payloads_are_rejected() {
    let err = generate_err("struct Foo { @payload a: [u8; 1], @payload b: [u8; 1] }");
    assert!(
        err.to_string()
            .contains("at most one @payload field")
    );
}

#[test]
fn payload_inside_conditional_is_rejected() {
    let err = generate_err("struct Foo { f: u8, if (f > 0) { @payload p: [u8; 1] } }");
    assert!(
        err.to_string()
            .contains("@payload cannot be applied inside a conditional")
    );
}

#[test]
fn discriminator_on_skip_is_rejected() {
    let err = generate_err("struct Foo { @skip @discriminator x: u8, @payload p: [u8; 1] }");
    assert!(
        err.to_string()
            .contains("@discriminator cannot be applied to a @skip field")
    );
}

#[test]
fn discriminator_wrong_arg_count_is_rejected() {
    let err = generate_err("struct Foo { @discriminator(1) x: u8 }");
    assert!(err.to_string().contains("requires exactly 0 argument"));
}

#[test]
fn array_size_unknown_field_is_rejected() {
    let err = generate_err("struct Foo { items: [u16; nope] }");
    assert!(
        err.to_string()
            .contains("expression 'nope' references field 'nope' which is unknown or not yet parsed")
    );
}

#[test]
fn array_size_non_numeric_field_is_rejected() {
    let err = generate_err("struct Inner { x: u8 } struct Foo { inner: Inner, items: [u8; inner] }");
    assert!(
        err.to_string()
            .contains("expression 'inner' references field 'inner' which is not a numeric field")
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
    assert!(
        err.to_string()
            .contains("expression 'kind' references field 'kind' which is unknown or not yet parsed")
    );
}

#[test]
fn union_non_numeric_argument_is_rejected() {
    let err = generate_err(
        r#"struct Inner { x: u8 }
        struct Foo {
            inner: Inner,
            payload: union(inner) {
                1 => A { x: u8 },
                _ => B { },
            },
        }"#,
    );
    assert!(
        err.to_string()
            .contains("expression 'inner' references field 'inner' which is not a numeric field")
    );
}

#[test]
fn union_without_catch_all_is_rejected() {
    let err = generate_err(
        r#"struct Packet {
            ty: u8,
            payload: union(ty) {
                1 => Echo { id: u16 },
            },
        }"#,
    );
    assert!(err.to_string().contains("union is not exhaustive"));
}

#[test]
fn union_with_wildcard_error_variant_is_exhaustive() {
    let code = generate(
        r#"error {
            UNKNOWN_TYPE,
        }

        struct Packet {
            ty: u8,
            payload: union(ty) {
                1 => Echo { id: u16 },
                _ => @error(UNKNOWN_TYPE),
            },
        }"#,
    );
    assert!(code.contains("Err(Error::UNKNOWN_TYPE)"));
}

#[test]
fn union_tuple_of_wildcards_is_exhaustive() {
    let code = generate(
        r#"struct Packet {
            ty: u8,
            code: u8,
            payload: union(ty, code) {
                (1, 1) => Echo { id: u16 },
                (_, _) => Unknown { },
            },
        }"#,
    );
    assert!(code.contains("(_, _) =>"));
}

#[test]
fn union_matcher_arity_mismatch_is_rejected() {
    let err = generate_err(
        r#"struct Packet {
            ty: u8,
            payload: union(ty) {
                (1, 2) => Echo { id: u16 },
                _ => Unknown { },
            },
        }"#,
    );
    assert!(
        err.to_string()
            .contains("matcher has 2 elements but union has 1 arguments")
    );
}

#[test]
fn union_literal_matcher_on_tuple_union_is_rejected() {
    let err = generate_err(
        r#"struct Packet {
            ty: u8,
            code: u8,
            payload: union(ty, code) {
                1 => Echo { id: u16 },
                _ => Unknown { },
            },
        }"#,
    );
    assert!(
        err.to_string()
            .contains("matcher has 1 elements but union has 2 arguments")
    );
}

#[test]
fn union_unknown_error_variant_is_rejected() {
    let err = generate_err(
        r#"struct Packet {
            ty: u8,
            payload: union(ty) {
                1 => Echo { id: u16 },
                _ => @error(UNKNOWN_TYPE),
            },
        }"#,
    );
    assert!(
        err.to_string()
            .contains("@error variant 'UNKNOWN_TYPE' is not declared in an error block")
    );
}

#[test]
fn union_error_variant_missing_field_is_rejected() {
    let err = generate_err(
        r#"error {
            UNKNOWN_TYPE { ty: u8, code: u8 },
        }

        struct Packet {
            ty: u8,
            payload: union(ty) {
                1 => Echo { id: u16 },
                _ => @error(UNKNOWN_TYPE { ty: ty }),
            },
        }"#,
    );
    assert!(
        err.to_string()
            .contains("@error variant 'UNKNOWN_TYPE' is missing field 'code'")
    );
}

#[test]
fn union_error_variant_unknown_field_is_rejected() {
    let err = generate_err(
        r#"error {
            UNKNOWN_TYPE { ty: u8 },
        }

        struct Packet {
            ty: u8,
            payload: union(ty) {
                1 => Echo { id: u16 },
                _ => @error(UNKNOWN_TYPE { ty: ty, extra: ty }),
            },
        }"#,
    );
    assert!(
        err.to_string()
            .contains("@error variant 'UNKNOWN_TYPE' has no declared field 'extra'")
    );
}

#[test]
fn unions_in_concat_get_distinct_names() {
    let code = generate(
        r#"struct Packet {
            a: u8,
            b: u8,
            pair: concat(
                union(a) { 1 => X { x: u8 }, _ => XOther { } },
                union(b) { 2 => Y { y: u16 }, _ => YOther { } }
            ),
        }"#,
    );
    assert!(code.contains("pub enum Packet_pair_0<'a>"));
    assert!(code.contains("pub enum Packet_pair_1<'a>"));
    assert!(code.contains("self.pair_0_union_check()?;"));
    assert!(code.contains("self.pair_1_union_check()?;"));
}

#[test]
fn error_struct_name_conflict_is_rejected() {
    let err = generate_err(
        r#"error {
            UNKNOWN_TYPE,
        }

        struct Error {
            ty: u8,
        }"#,
    );
    assert!(
        err.to_string()
            .contains("struct name 'Error' conflicts with the generated error enum")
    );
}

#[test]
fn error_struct_name_without_error_block_is_allowed() {
    let code = generate("struct Error { ty: u8 }");
    assert!(code.contains("pub struct Error<'a>"));
}

#[test]
fn parse_error_variant_name_is_rejected() {
    let err = generate_err("error { Parse }");
    assert!(
        err.to_string()
            .contains("error variant 'Parse' is reserved for wrapped parse errors")
    );
}

#[test]
fn duplicate_error_block_is_rejected() {
    let err = generate_err("error { A } error { B }");
    assert!(err.to_string().contains("duplicate error block"));
}

#[test]
fn duplicate_error_variant_is_rejected() {
    let err = generate_err("error { A, A }");
    assert!(err.to_string().contains("duplicate error variant 'A'"));
}

#[test]
fn union_variant_validation_is_generated_in_variant_parse() {
    let code = generate(
        r#"struct Packet {
            ty: u8,
            payload: union(ty) {
                1 => Echo { version = 4 },
                _ => Unknown { },
            },
        }"#,
    );
    assert!(code.contains("fn version_validate"));
    assert!(code.contains("self.version_validate()?;"));
    assert!(code.contains("me.version_recoverable_check()?;"));
}

#[test]
fn array_size_forward_reference_is_rejected() {
    let err = generate_err("struct Foo { data: [u8; later], later: u8 }");
    assert!(
        err.to_string()
            .contains("references field 'later' which is unknown or not yet parsed")
    );
}

#[test]
fn array_size_bool_expr_is_rejected() {
    let err = generate_err("struct Foo { n: u8, data: [u8; n == 1] }");
    assert!(
        err.to_string()
            .contains("is a boolean but a number is required")
    );
}

#[test]
fn array_size_string_is_rejected() {
    let err = generate_err(r#"struct Foo { data: [u8; "two"] }"#);
    assert!(
        err.to_string()
            .contains("is a string but a number is required")
    );
}

#[test]
fn array_size_nested_path_is_rejected() {
    let err = generate_err("struct Foo { a: u8, data: [u8; a.len] }");
    assert!(err.to_string().contains("nested path 'a.len'"));
}

#[test]
fn array_size_const_overflow_is_rejected() {
    let err = generate_err("struct Foo { data: [u8; 4294967296 * 4294967296] }");
    assert!(err.to_string().contains("overflows"));
}

#[test]
fn array_size_division_by_zero_is_rejected() {
    let err = generate_err("struct Foo { data: [u8; 8 / 0] }");
    assert!(err.to_string().contains("divides by zero"));
}

#[test]
fn array_size_dynamic_divisor_is_rejected() {
    let err = generate_err("struct Foo { n: u8, data: [u8; 8 / n] }");
    assert!(
        err.to_string()
            .contains("divides by a non-constant value")
    );
}

#[test]
fn same_array_field_name_in_two_structs_does_not_collide() {
    let code = generate(
        "struct First { n: u8, xs: [u8; n] } struct Second { n: u8, xs: [u16; n] }",
    );
    assert!(code.contains("First_xs_Iterator"));
    assert!(code.contains("Second_xs_Iterator"));
}

#[test]
fn array_size_const_expr_is_folded() {
    let code = generate("struct Foo { data: [u8; 2 * 3 + 1] }");
    assert!(code.contains("count: 7usize"));
}

#[test]
fn array_size_subtraction_saturates() {
    let code = generate("struct Foo { n: u8, data: [u8; n - 1] }");
    assert!(code.contains("(self.n() as usize).saturating_sub(1usize)"));
}

fn constraint_expr<'a>(ast: &'a [binparse_dsl::Definition<'a>]) -> &'a binparse_dsl::Expr<'a> {
    let binparse_dsl::Definition::Struct(s) = &ast[0] else {
        panic!("expected struct");
    };
    let binparse_dsl::StructItem::Field(f) = &s.items[0] else {
        panic!("expected field");
    };
    let binparse_dsl::FieldValue::Constraint(e) = &f.value else {
        panic!("expected constraint");
    };
    e
}

fn numeric_done_fields() -> Vec<crate::struct_::DoneField> {
    ["n", "m"]
        .into_iter()
        .map(|name| crate::struct_::DoneField {
            name: name.to_string(),
            field_type: crate::struct_::DoneFieldType::Primitive,
            conditional: false,
        })
        .collect()
}

#[test]
fn lower_bool_expr() {
    let ast = binparse_dsl_parse::parse_str("struct Foo { c = n == 1 && m < 2 }").unwrap();
    let lowered = crate::expr::lower(
        constraint_expr(&ast),
        crate::expr::ExprType::Bool,
        &numeric_done_fields(),
    )
    .unwrap();
    let expected = quote::quote! {
        ((((self.n() as usize) == (1usize))) && (((self.m() as usize) < (2usize))))
    };
    assert_eq!(lowered.tokens.to_string(), expected.to_string());
}

#[test]
fn lower_bool_rejects_numeric_expr() {
    let ast = binparse_dsl_parse::parse_str("struct Foo { c = n + 1 }").unwrap();
    let err = crate::expr::lower(
        constraint_expr(&ast),
        crate::expr::ExprType::Bool,
        &numeric_done_fields(),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("expression '(n + 1)' is a number but a boolean is required")
    );
}

#[test]
fn golden_constant_fields() {
    assert_generated_eq(
        "struct Magic { magic = xc0de, flags = b101 }",
        r#"
pub struct Magic<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Magic<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.magic_fatal_check()?;
        me.magic_recoverable_check()?;
        me.flags_fatal_check()?;
        me.flags_recoverable_check()?;
        let len = me.flags_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.magic_present_node());
        }
        {
            let me = self;
            children.push(me.flags_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Magic",
                "Magic",
                0usize..self.flags_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.magic_fatal_check() {
                Err(error) => {
                    let start = me.magic_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "magic",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.magic_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.magic_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "magic",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.magic_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.flags_fatal_check() {
                Err(error) => {
                    let start = me.flags_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "flags",
                                    "b<3>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.flags_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.flags_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "flags",
                                            "b<3>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.flags_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Magic",
                "Magic",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn magic_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.magic_bit_range();
        ::binparse::FieldNode::new(
            "magic",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.magic())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn magic(&self) -> u16 {
        u16::from_be_bytes(self.data[0usize..2usize].try_into().unwrap())
    }
    pub fn magic_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
    }
    pub fn magic_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn magic_bit_range(&self) -> ::core::ops::Range<usize> {
        self.magic_start_offset().bits()..self.magic_end_offset().bits()
    }
    #[allow(dead_code)]
    fn magic_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.magic_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(clippy::unnecessary_cast)]
    fn magic_validate(&self) -> Result<(), binparse::ParseError> {
        if self.magic() != 49374 {
            return Err(binparse::ParseError::ValidationFailed {
                field: "Magic.magic",
                actual: self.magic() as u128,
            });
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn magic_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        self.magic_validate()?;
        Ok(())
    }
    #[allow(dead_code)]
    fn flags_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.flags_bit_range();
        ::binparse::FieldNode::new(
            "flags",
            "b<3>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.flags())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn flags(&self) -> u8 {
        (self.data[2usize] >> 5usize) & 7u8
    }
    pub fn flags_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 3usize,
        }
    }
    pub fn flags_start_offset(&self) -> binparse::Len {
        self.magic_end_offset()
    }
    pub fn flags_bit_range(&self) -> ::core::ops::Range<usize> {
        self.flags_start_offset().bits()..self.flags_end_offset().bits()
    }
    #[allow(dead_code)]
    fn flags_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.flags_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(clippy::unnecessary_cast)]
    fn flags_validate(&self) -> Result<(), binparse::ParseError> {
        if self.flags() != 5 {
            return Err(binparse::ParseError::ValidationFailed {
                field: "Magic.flags",
                actual: self.flags() as u128,
            });
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn flags_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        self.flags_validate()?;
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Magic<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Magic::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Magic::handoff(self)
    }
}

"#,
    );
}

#[test]
fn golden_check_and_range() {
    assert_generated_eq(
        "struct Checked { n: u8, @range(1, n + 1) @check(m >= n) m: u8 }",
        r#"
pub struct Checked<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Checked<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.n_fatal_check()?;
        me.n_recoverable_check()?;
        me.m_fatal_check()?;
        me.m_recoverable_check()?;
        let len = me.m_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.n_present_node());
        }
        {
            let me = self;
            children.push(me.m_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Checked",
                "Checked",
                0usize..self.m_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.n_fatal_check() {
                Err(error) => {
                    let start = me.n_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "n",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.n_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.n_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "n",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.n_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.m_fatal_check() {
                Err(error) => {
                    let start = me.m_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "m",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.m_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.m_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "m",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.m_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Checked",
                "Checked",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn n_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.n_bit_range();
        ::binparse::FieldNode::new(
            "n",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.n())),
        )
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
    pub fn n_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn n_bit_range(&self) -> ::core::ops::Range<usize> {
        self.n_start_offset().bits()..self.n_end_offset().bits()
    }
    #[allow(dead_code)]
    fn n_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.n_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn n_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn m_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.m_bit_range();
        ::binparse::FieldNode::new(
            "m",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.m())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn m(&self) -> u8 {
        self.data[1usize]
    }
    pub fn m_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
    }
    pub fn m_start_offset(&self) -> binparse::Len {
        self.n_end_offset()
    }
    pub fn m_bit_range(&self) -> ::core::ops::Range<usize> {
        self.m_start_offset().bits()..self.m_end_offset().bits()
    }
    #[allow(dead_code)]
    fn m_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.m_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(clippy::unnecessary_cast)]
    fn m_validate(&self) -> Result<(), binparse::ParseError> {
        if !((1usize)..=((self.n() as usize).saturating_add(1usize)))
            .contains(&(self.m() as usize))
        {
            return Err(binparse::ParseError::ValidationFailed {
                field: "Checked.m",
                actual: self.m() as u128,
            });
        }
        if !((self.m() as usize) >= (self.n() as usize)) {
            return Err(binparse::ParseError::ValidationFailed {
                field: "Checked.m",
                actual: self.m() as u128,
            });
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn m_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        self.m_validate()?;
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Checked<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Checked::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Checked::handoff(self)
    }
}

"#,
    );
}

#[test]
fn constant_decimal_infers_smallest_type() {
    let code = generate("struct Foo { small = 10, medium = 65536 }");
    assert!(code.contains("pub fn small(&self) -> u8"));
    assert!(code.contains("pub fn medium(&self) -> u32"));
}

#[test]
fn constant_hex_infers_type_from_width() {
    let code = generate("struct Foo { a = x0f, b = x0102030405060708 }");
    assert!(code.contains("pub fn a(&self) -> u8"));
    assert!(code.contains("pub fn b(&self) -> u64"));
}

#[test]
fn validate_attribute_is_an_alias_for_check() {
    let code = generate("struct Foo { @validate(n == 1) n: u8 }");
    assert!(code.contains("fn n_validate"));
    assert!(code.contains("self.n_validate()?"));
}

#[test]
fn constant_binary_too_wide_is_rejected() {
    let err = generate_err("struct Foo { f = b101010101 }");
    assert!(
        err.to_string()
            .contains("constant field literal width 9 is not supported")
    );
}

#[test]
fn constant_field_rejects_endian_on_single_byte() {
    let err = generate_err("struct Foo { @endian(little) f = x01 }");
    assert!(
        err.to_string()
            .contains("@endian cannot be applied to single-byte integers")
    );
}

#[test]
fn check_on_struct_ref_is_rejected() {
    let err = generate_err("struct Inner { x: u8 } struct Foo { @check(1 == 1) inner: Inner }");
    assert!(
        err.to_string()
            .contains("@check and @range can only be applied to primitive and bitfield fields")
    );
}

#[test]
fn range_on_array_is_rejected() {
    let err = generate_err("struct Foo { @range(1, 2) xs: [u8; 4] }");
    assert!(
        err.to_string()
            .contains("@check and @range can only be applied to primitive and bitfield fields")
    );
}

#[test]
fn check_with_numeric_expr_is_rejected() {
    let err = generate_err("struct Foo { @check(n + 1) n: u8 }");
    assert!(
        err.to_string()
            .contains("is a number but a boolean is required")
    );
}

#[test]
fn range_with_bool_expr_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @range(n == 1, 5) m: u8 }");
    assert!(
        err.to_string()
            .contains("is a boolean but a number is required")
    );
}

#[test]
fn check_unknown_field_is_rejected() {
    let err = generate_err("struct Foo { @check(later == 1) n: u8, later: u8 }");
    assert!(
        err.to_string()
            .contains("references field 'later' which is unknown or not yet parsed")
    );
}

#[test]
fn check_wrong_arg_count_is_rejected() {
    let err = generate_err("struct Foo { @check(n == 1, n == 2) n: u8 }");
    assert!(
        err.to_string()
            .contains("@check requires exactly 1 argument(s), got 2")
    );
}

#[test]
fn range_wrong_arg_count_is_rejected() {
    let err = generate_err("struct Foo { @range(1) n: u8 }");
    assert!(
        err.to_string()
            .contains("@range requires exactly 2 argument(s), got 1")
    );
}

#[test]
fn range_min_greater_than_max_is_rejected() {
    let err = generate_err("struct Foo { @range(5, 1) n: u8 }");
    assert!(
        err.to_string()
            .contains("@range minimum 5 is greater than maximum 1")
    );
}

#[test]
fn lower_bool_rejects_numeric_logic_operand() {
    let ast = binparse_dsl_parse::parse_str("struct Foo { c = n == 1 && 2 }").unwrap();
    let err = crate::expr::lower(
        constraint_expr(&ast),
        crate::expr::ExprType::Bool,
        &numeric_done_fields(),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("is a number but a boolean is required")
    );
}

#[test]
fn golden_conditional_fields() {
    assert_generated_eq(
        r#"
        struct Cond {
            n: u8,
            if (n == 1) {
                x: u16,
            } else {
                y: u8,
            }
            tail: u8,
        }
        "#,
        r#"
pub struct Cond<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Cond<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.n_fatal_check()?;
        me.n_recoverable_check()?;
        if me.conditional_0_present() {
            me.x_fatal_check()?;
            me.x_recoverable_check()?;
        }
        if me.conditional_0_absent() {
            me.y_fatal_check()?;
            me.y_recoverable_check()?;
        }
        me.tail_fatal_check()?;
        me.tail_recoverable_check()?;
        let len = me.tail_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.n_present_node());
        }
        {
            let me = self;
            if me.conditional_0_present() {
                children.push(me.x_present_node());
            } else {
                children.push(me.x_absent_node());
            }
        }
        {
            let me = self;
            if me.conditional_0_absent() {
                children.push(me.y_present_node());
            } else {
                children.push(me.y_absent_node());
            }
        }
        {
            let me = self;
            children.push(me.tail_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Cond",
                "Cond",
                0usize..self.tail_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.n_fatal_check() {
                Err(error) => {
                    let start = me.n_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "n",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.n_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.n_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "n",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.n_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            if me.conditional_0_present() {
                match me.x_fatal_check() {
                    Err(error) => {
                        let start = me.x_start_offset().bits();
                        children
                            .push(
                                ::binparse::FieldNode::new(
                                        "x",
                                        "u16",
                                        start..start,
                                        ::binparse::Value::Opaque,
                                    )
                                    .with_status(::binparse::Status::Error(error)),
                            );
                        fatal = Some(error);
                    }
                    Ok(()) => {
                        match me.x_recoverable_check() {
                            Err(error) => {
                                let bit_range = me.x_bit_range();
                                children
                                    .push(
                                        ::binparse::FieldNode::new(
                                                "x",
                                                "u16",
                                                bit_range,
                                                ::binparse::Value::Opaque,
                                            )
                                            .with_status(::binparse::Status::Error(error)),
                                    );
                            }
                            Ok(()) => {
                                children.push(me.x_present_node());
                            }
                        }
                    }
                }
            } else {
                children.push(me.x_absent_node());
            }
        }
        if fatal.is_none() {
            if me.conditional_0_absent() {
                match me.y_fatal_check() {
                    Err(error) => {
                        let start = me.y_start_offset().bits();
                        children
                            .push(
                                ::binparse::FieldNode::new(
                                        "y",
                                        "u8",
                                        start..start,
                                        ::binparse::Value::Opaque,
                                    )
                                    .with_status(::binparse::Status::Error(error)),
                            );
                        fatal = Some(error);
                    }
                    Ok(()) => {
                        match me.y_recoverable_check() {
                            Err(error) => {
                                let bit_range = me.y_bit_range();
                                children
                                    .push(
                                        ::binparse::FieldNode::new(
                                                "y",
                                                "u8",
                                                bit_range,
                                                ::binparse::Value::Opaque,
                                            )
                                            .with_status(::binparse::Status::Error(error)),
                                    );
                            }
                            Ok(()) => {
                                children.push(me.y_present_node());
                            }
                        }
                    }
                }
            } else {
                children.push(me.y_absent_node());
            }
        }
        if fatal.is_none() {
            match me.tail_fatal_check() {
                Err(error) => {
                    let start = me.tail_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "tail",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.tail_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.tail_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "tail",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.tail_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Cond",
                "Cond",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn n_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.n_bit_range();
        ::binparse::FieldNode::new(
            "n",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.n())),
        )
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
    pub fn n_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn n_bit_range(&self) -> ::core::ops::Range<usize> {
        self.n_start_offset().bits()..self.n_end_offset().bits()
    }
    #[allow(dead_code)]
    fn n_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.n_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn n_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code, unused_parens)]
    fn conditional_0_present(&self) -> bool {
        ((self.n() as usize) == (1usize))
    }
    #[allow(dead_code)]
    fn x_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.x_bit_range();
        ::binparse::FieldNode::new(
            "x",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.x_raw())),
        )
    }
    #[allow(dead_code)]
    fn x_absent_node(&self) -> ::binparse::FieldNode<'a> {
        let start = self.x_start_offset().bits();
        ::binparse::FieldNode::new("x", "u16", start..start, ::binparse::Value::Absent)
            .hide()
    }
    #[allow(clippy::identity_op)]
    fn x_raw(&self) -> u16 {
        u16::from_be_bytes(self.data[1usize..3usize].try_into().unwrap())
    }
    pub fn x(&self) -> Option<u16> {
        if self.conditional_0_present() { Some(self.x_raw()) } else { None }
    }
    pub fn x_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 3usize,
            bit: 0usize,
        }
    }
    pub fn x_start_offset(&self) -> binparse::Len {
        self.n_end_offset()
    }
    pub fn x_bit_range(&self) -> ::core::ops::Range<usize> {
        self.x_start_offset().bits()..self.x_end_offset().bits()
    }
    #[allow(dead_code)]
    fn x_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.x_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn x_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code, unused_parens)]
    fn conditional_0_absent(&self) -> bool {
        !((self.n() as usize) == (1usize))
    }
    #[allow(dead_code)]
    fn y_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.y_bit_range();
        ::binparse::FieldNode::new(
            "y",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.y_raw())),
        )
    }
    #[allow(dead_code)]
    fn y_absent_node(&self) -> ::binparse::FieldNode<'a> {
        let start = self.y_start_offset().bits();
        ::binparse::FieldNode::new("y", "u8", start..start, ::binparse::Value::Absent)
            .hide()
    }
    #[allow(clippy::identity_op)]
    fn y_raw(&self) -> u8 {
        self.data[1usize]
    }
    pub fn y(&self) -> Option<u8> {
        if self.conditional_0_absent() { Some(self.y_raw()) } else { None }
    }
    pub fn y_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
    }
    pub fn y_start_offset(&self) -> binparse::Len {
        self.n_end_offset()
    }
    pub fn y_bit_range(&self) -> ::core::ops::Range<usize> {
        self.y_start_offset().bits()..self.y_end_offset().bits()
    }
    #[allow(dead_code)]
    fn y_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.y_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn y_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    fn conditional_0_end_offset(&self) -> binparse::Len {
        if self.conditional_0_present() {
            self.x_end_offset()
        } else {
            self.y_end_offset()
        }
    }
    #[allow(dead_code)]
    fn tail_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.tail_bit_range();
        ::binparse::FieldNode::new(
            "tail",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.tail())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn tail(&self) -> u8 {
        {
            let offset = self.conditional_0_end_offset();
            debug_assert!(offset.bit == 0, "primitive requires byte alignment");
            self.data[offset.byte]
        }
    }
    pub fn tail_end_offset(&self) -> binparse::Len {
        ({ self.conditional_0_end_offset() })
            + ::binparse::Len {
                byte: 1usize,
                bit: 0usize,
            }
    }
    pub fn tail_start_offset(&self) -> binparse::Len {
        self.conditional_0_end_offset()
    }
    pub fn tail_bit_range(&self) -> ::core::ops::Range<usize> {
        self.tail_start_offset().bits()..self.tail_end_offset().bits()
    }
    #[allow(dead_code)]
    fn tail_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.tail_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn tail_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Cond<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Cond::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Cond::handoff(self)
    }
}

"#,
    );
}

#[test]
fn conditional_field_reference_is_rejected() {
    let err = generate_err("struct Foo { n: u8, if (n == 1) { m: u8 } data: [u8; m] }");
    assert!(
        err.to_string()
            .contains("expression 'm' references conditional field 'm' which may be absent")
    );
}

#[test]
fn conditional_intra_branch_reference_is_rejected() {
    let err = generate_err("struct Foo { n: u8, if (n == 1) { m: u8, data: [u8; m] } }");
    assert!(
        err.to_string()
            .contains("expression 'm' references conditional field 'm' which may be absent")
    );
}

#[test]
fn conditional_numeric_condition_is_rejected() {
    let err = generate_err("struct Foo { n: u8, if (n) { m: u8 } }");
    assert!(
        err.to_string()
            .contains("expression 'n' is a number but a boolean is required")
    );
}

#[test]
fn conditional_forward_reference_is_rejected() {
    let err = generate_err("struct Foo { if (later == 1) { m: u8 } later: u8 }");
    assert!(
        err.to_string()
            .contains("expression '(later == 1)' references field 'later' which is unknown or not yet parsed")
    );
}

#[test]
fn unsized_array_without_strategy_is_rejected() {
    let err = generate_err("struct Foo { data: [u8] }");
    assert!(
        err.to_string()
            .contains("array without size requires @until, @greedy, or @hook")
    );
}

#[test]
fn until_on_sized_array_is_rejected() {
    let err = generate_err("struct Foo { @until(x00) data: [u8; 4] }");
    assert!(
        err.to_string()
            .contains("@until requires an array without an explicit size")
    );
}

#[test]
fn greedy_on_sized_array_is_rejected() {
    let err = generate_err("struct Foo { @greedy(unsafe_eof) data: [u8; 4] }");
    assert!(
        err.to_string()
            .contains("@greedy requires an array without an explicit size")
    );
}

#[test]
fn until_on_non_array_is_rejected() {
    let err = generate_err("struct Foo { @until(x00) data: u8 }");
    assert!(
        err.to_string()
            .contains("@until can only be applied to array fields")
    );
}

#[test]
fn max_iter_on_non_array_is_rejected() {
    let err = generate_err("struct Foo { @max_iter(4) data: u8 }");
    assert!(
        err.to_string()
            .contains("@max_iter can only be applied to array fields")
    );
}

#[test]
fn until_with_greedy_is_rejected() {
    let err = generate_err("struct Foo { @until(x00) @greedy(unsafe_eof) data: [u8] }");
    assert!(
        err.to_string()
            .contains("@until and @greedy cannot be combined")
    );
}

#[test]
fn greedy_with_hook_is_rejected() {
    let err = generate_err("struct Foo { @hook(f, u8) @greedy(unsafe_eof) data: [u8] }");
    assert!(
        err.to_string()
            .contains("@greedy cannot be combined with @hook")
    );
}

#[test]
fn hook_on_non_u8_vla_is_rejected() {
    let err = generate_err("struct Foo { @hook(f, u8) data: [u16] }");
    assert!(err.to_string().contains("@hook on VLA requires [u8] type"));
}

#[test]
fn invalid_greedy_value_is_rejected() {
    let err = generate_err("struct Foo { @greedy(eof) data: [u8] }");
    assert!(
        err.to_string()
            .contains("@greedy argument must be 'unsafe_eof', got 'eof'")
    );
}

#[test]
fn until_sentinel_too_wide_is_rejected() {
    let err = generate_err("struct Foo { @until(x0100) data: [u8] }");
    assert!(
        err.to_string()
            .contains("@until sentinel must be an integer literal fitting in one byte")
    );
}

#[test]
fn greedy_dynamic_elem_without_max_iter_is_rejected() {
    let err = generate_err(
        "struct Opt { kind: u8, if (kind > 0) { body: u8 } } struct Foo { @greedy(unsafe_eof) opts: [Opt] }",
    );
    assert!(
        err.to_string()
            .contains("@greedy with dynamic-length elements requires @max_iter")
    );
}

#[test]
fn greedy_zero_sized_elem_is_rejected() {
    let err = generate_err("struct Empty { } struct Foo { @greedy(unsafe_eof) xs: [Empty] }");
    assert!(err.to_string().contains("@greedy element type has zero length"));
}

#[test]
fn golden_padding_and_alignment() {
    assert_generated_eq(
        "struct Padded { a: u8, @pad(2) b: u8, @pad_to(4) c: u16, @align(2) d: u16 }",
        r#"
pub struct Padded<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Padded<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.a_fatal_check()?;
        me.a_recoverable_check()?;
        me.b_fatal_check()?;
        me.b_recoverable_check()?;
        me.c_fatal_check()?;
        me.c_recoverable_check()?;
        me.d_fatal_check()?;
        me.d_recoverable_check()?;
        let len = me.d_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.a_present_node());
        }
        {
            let me = self;
            if let Some(node) = me.b_pad_node() {
                children.push(node);
            }
            children.push(me.b_present_node());
        }
        {
            let me = self;
            if let Some(node) = me.c_pad_node() {
                children.push(node);
            }
            children.push(me.c_present_node());
        }
        {
            let me = self;
            children.push(me.d_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Padded",
                "Padded",
                0usize..self.d_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.a_fatal_check() {
                Err(error) => {
                    let start = me.a_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "a",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.a_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.a_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "a",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.a_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            if let Some(node) = me.b_pad_node() {
                children.push(node);
            }
            match me.b_fatal_check() {
                Err(error) => {
                    let start = me.b_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "b",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.b_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.b_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "b",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.b_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            if let Some(node) = me.c_pad_node() {
                children.push(node);
            }
            match me.c_fatal_check() {
                Err(error) => {
                    let start = me.c_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "c",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.c_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.c_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "c",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.c_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.d_fatal_check() {
                Err(error) => {
                    let start = me.d_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "d",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.d_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.d_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "d",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.d_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Padded",
                "Padded",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn a_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.a_bit_range();
        ::binparse::FieldNode::new(
            "a",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.a())),
        )
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
    pub fn a_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn a_bit_range(&self) -> ::core::ops::Range<usize> {
        self.a_start_offset().bits()..self.a_end_offset().bits()
    }
    #[allow(dead_code)]
    fn a_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.a_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn a_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn b_pad_node(&self) -> Option<::binparse::FieldNode<'a>> {
        let bit_range = self.a_end_offset().bits()..self.b_start_offset().bits();
        if bit_range.start < bit_range.end {
            Some(
                ::binparse::FieldNode::new(
                        "b_pad",
                        "pad",
                        bit_range.clone(),
                        ::binparse::Value::bytes(self.data, &bit_range),
                    )
                    .hide(),
            )
        } else {
            None
        }
    }
    #[allow(dead_code)]
    fn b_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.b_bit_range();
        ::binparse::FieldNode::new(
            "b",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.b())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn b(&self) -> u8 {
        self.data[3usize]
    }
    pub fn b_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 4usize,
            bit: 0usize,
        }
    }
    pub fn b_start_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 3usize,
            bit: 0usize,
        }
    }
    pub fn b_bit_range(&self) -> ::core::ops::Range<usize> {
        self.b_start_offset().bits()..self.b_end_offset().bits()
    }
    #[allow(dead_code)]
    fn b_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.b_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn b_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn c_pad_node(&self) -> Option<::binparse::FieldNode<'a>> {
        let bit_range = self.b_end_offset().bits()..self.c_start_offset().bits();
        if bit_range.start < bit_range.end {
            Some(
                ::binparse::FieldNode::new(
                        "c_pad",
                        "pad",
                        bit_range.clone(),
                        ::binparse::Value::bytes(self.data, &bit_range),
                    )
                    .hide(),
            )
        } else {
            None
        }
    }
    #[allow(dead_code)]
    fn c_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.c_bit_range();
        ::binparse::FieldNode::new(
            "c",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.c())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn c(&self) -> u16 {
        u16::from_be_bytes(self.data[4usize..6usize].try_into().unwrap())
    }
    pub fn c_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 6usize,
            bit: 0usize,
        }
    }
    pub fn c_start_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 4usize,
            bit: 0usize,
        }
    }
    pub fn c_bit_range(&self) -> ::core::ops::Range<usize> {
        self.c_start_offset().bits()..self.c_end_offset().bits()
    }
    #[allow(dead_code)]
    fn c_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.c_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn c_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn d_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.d_bit_range();
        ::binparse::FieldNode::new(
            "d",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.d())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn d(&self) -> u16 {
        u16::from_be_bytes(self.data[6usize..8usize].try_into().unwrap())
    }
    pub fn d_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 8usize,
            bit: 0usize,
        }
    }
    pub fn d_start_offset(&self) -> binparse::Len {
        self.c_end_offset()
    }
    pub fn d_bit_range(&self) -> ::core::ops::Range<usize> {
        self.d_start_offset().bits()..self.d_end_offset().bits()
    }
    #[allow(dead_code)]
    fn d_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.d_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn d_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Padded<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Padded::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Padded::handoff(self)
    }
}

"#,
    );
}

#[test]
fn golden_skip_fields() {
    assert_generated_eq(
        "struct Skipped { @skip reserved: b<3>, flags: b<5>, pair: concat(b<4>, @skip b<4>) }",
        r#"
pub struct Skipped<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Skipped<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.reserved_fatal_check()?;
        me.reserved_recoverable_check()?;
        me.flags_fatal_check()?;
        me.flags_recoverable_check()?;
        me.pair_fatal_check()?;
        me.pair_recoverable_check()?;
        let len = me.pair_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.reserved_present_node());
        }
        {
            let me = self;
            children.push(me.flags_present_node());
        }
        {
            let me = self;
            children.push(me.pair_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Skipped",
                "Skipped",
                0usize..self.pair_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.reserved_fatal_check() {
                Err(error) => {
                    let start = me.reserved_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "reserved",
                                    "b<3>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.reserved_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.reserved_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "reserved",
                                            "b<3>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.reserved_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.flags_fatal_check() {
                Err(error) => {
                    let start = me.flags_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "flags",
                                    "b<5>",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.flags_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.flags_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "flags",
                                            "b<5>",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.flags_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.pair_fatal_check() {
                Err(error) => {
                    let start = me.pair_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "pair",
                                    "concat",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.pair_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.pair_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "pair",
                                            "concat",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.pair_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Skipped",
                "Skipped",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn reserved_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.reserved_bit_range();
        ::binparse::FieldNode::new(
                "reserved",
                "b<3>",
                bit_range.clone(),
                ::binparse::Value::UInt(u128::from(self.reserved())),
            )
            .hide()
    }
    #[allow(dead_code)]
    #[allow(clippy::identity_op)]
    fn reserved(&self) -> u8 {
        (self.data[0usize] >> 5usize) & 7u8
    }
    #[allow(dead_code)]
    fn reserved_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 0usize,
            bit: 3usize,
        }
    }
    #[allow(dead_code)]
    fn reserved_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    #[allow(dead_code)]
    fn reserved_bit_range(&self) -> ::core::ops::Range<usize> {
        self.reserved_start_offset().bits()..self.reserved_end_offset().bits()
    }
    #[allow(dead_code)]
    fn reserved_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.reserved_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn reserved_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn flags_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.flags_bit_range();
        ::binparse::FieldNode::new(
            "flags",
            "b<5>",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.flags())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn flags(&self) -> u8 {
        (self.data[0usize] >> 0usize) & 31u8
    }
    pub fn flags_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
    }
    pub fn flags_start_offset(&self) -> binparse::Len {
        self.reserved_end_offset()
    }
    pub fn flags_bit_range(&self) -> ::core::ops::Range<usize> {
        self.flags_start_offset().bits()..self.flags_end_offset().bits()
    }
    #[allow(dead_code)]
    fn flags_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.flags_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn flags_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn pair_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.pair_bit_range();
        {
            let mut item_nodes = ::std::vec::Vec::new();
            {
                let bit_range = 8usize..12usize;
                item_nodes
                    .push(
                        ::binparse::FieldNode::new(
                            "pair_0",
                            "b<4>",
                            bit_range.clone(),
                            ::binparse::Value::UInt(u128::from(self.pair_0())),
                        ),
                    );
            }
            {
                let bit_range = 12usize..16usize;
                item_nodes
                    .push(
                        ::binparse::FieldNode::new(
                                "pair_1",
                                "b<4>",
                                bit_range.clone(),
                                ::binparse::Value::UInt(u128::from(self.pair_1())),
                            )
                            .hide(),
                    );
            }
            ::binparse::FieldNode::new(
                    "pair",
                    "concat",
                    bit_range.clone(),
                    ::binparse::Value::Struct,
                )
                .with_children(item_nodes)
        }
    }
    #[allow(clippy::identity_op)]
    pub fn pair_0(&self) -> u8 {
        (self.data[1usize] >> 4usize) & 15u8
    }
    #[allow(dead_code)]
    #[allow(clippy::identity_op)]
    fn pair_1(&self) -> u8 {
        (self.data[1usize] >> 0usize) & 15u8
    }
    #[allow(clippy::identity_op)]
    pub fn pair(&self) -> (u8,) {
        (self.pair_0(),)
    }
    pub fn pair_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
    }
    pub fn pair_start_offset(&self) -> binparse::Len {
        self.flags_end_offset()
    }
    pub fn pair_bit_range(&self) -> ::core::ops::Range<usize> {
        self.pair_start_offset().bits()..self.pair_end_offset().bits()
    }
    #[allow(dead_code)]
    fn pair_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.pair_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn pair_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Skipped<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Skipped::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Skipped::handoff(self)
    }
}

"#,
    );
}

#[test]
fn align_on_misaligned_fixed_offset_is_rejected() {
    let err = generate_err("struct Foo { a: u8, @align(2) b: u8 }");
    assert!(
        err.to_string()
            .contains("@align(2) field starts at misaligned offset")
    );
}

#[test]
fn align_on_unaligned_bit_offset_is_rejected() {
    let err = generate_err("struct Foo { a: b<3>, @align(1) b: u8 }");
    assert!(
        err.to_string()
            .contains("@align(1) field starts at misaligned offset")
    );
}

#[test]
fn pad_with_pad_to_is_rejected() {
    let err = generate_err("struct Foo { @pad(1) @pad_to(4) a: u8 }");
    assert!(
        err.to_string()
            .contains("@pad and @pad_to cannot be combined")
    );
}

#[test]
fn zero_padding_arg_is_rejected() {
    let err = generate_err("struct Foo { @align(0) a: u8 }");
    assert!(
        err.to_string()
            .contains("@align argument must be a positive integer literal")
    );
}

#[test]
fn non_literal_padding_arg_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @pad(n) a: u8 }");
    assert!(
        err.to_string()
            .contains("@pad argument must be a positive integer literal")
    );
}

#[test]
fn skip_with_args_is_rejected() {
    let err = generate_err("struct Foo { @skip(1) a: u8 }");
    assert!(
        err.to_string()
            .contains("@skip requires exactly 0 argument(s), got 1")
    );
}

#[test]
fn pad_wrong_arg_count_is_rejected() {
    let err = generate_err("struct Foo { @pad_to(1, 2) a: u8 }");
    assert!(
        err.to_string()
            .contains("@pad_to requires exactly 1 argument(s), got 2")
    );
}

#[test]
fn golden_len_bounded_struct_ref() {
    assert_generated_eq(
        "struct Inner { a: u8, b: u16 }
         struct Tlv { tag: u8, len: u8, @len(len) value: Inner, after: u8 }",
        r#"
pub struct Tlv<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Tlv<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.tag_fatal_check()?;
        me.tag_recoverable_check()?;
        me.len_fatal_check()?;
        me.len_recoverable_check()?;
        me.value_fatal_check()?;
        me.value_recoverable_check()?;
        me.after_fatal_check()?;
        me.after_recoverable_check()?;
        let len = me.after_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.tag_present_node());
        }
        {
            let me = self;
            children.push(me.len_present_node());
        }
        {
            let me = self;
            children.push(me.value_present_node());
        }
        {
            let me = self;
            children.push(me.after_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Tlv",
                "Tlv",
                0usize..self.after_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.tag_fatal_check() {
                Err(error) => {
                    let start = me.tag_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "tag",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.tag_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.tag_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "tag",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.tag_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.len_fatal_check() {
                Err(error) => {
                    let start = me.len_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "len",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.len_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.len_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "len",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.len_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.value_fatal_check() {
                Err(error) => {
                    let start = me.value_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "value",
                                    "Inner",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.value_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.value_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "value",
                                            "Inner",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.value_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.after_fatal_check() {
                Err(error) => {
                    let start = me.after_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "after",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.after_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.after_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "after",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.after_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Tlv",
                "Tlv",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn tag_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.tag_bit_range();
        ::binparse::FieldNode::new(
            "tag",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.tag())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn tag(&self) -> u8 {
        self.data[0usize]
    }
    pub fn tag_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
    }
    pub fn tag_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn tag_bit_range(&self) -> ::core::ops::Range<usize> {
        self.tag_start_offset().bits()..self.tag_end_offset().bits()
    }
    #[allow(dead_code)]
    fn tag_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.tag_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn tag_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn len_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.len_bit_range();
        ::binparse::FieldNode::new(
            "len",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.len())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn len(&self) -> u8 {
        self.data[1usize]
    }
    pub fn len_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
    }
    pub fn len_start_offset(&self) -> binparse::Len {
        self.tag_end_offset()
    }
    pub fn len_bit_range(&self) -> ::core::ops::Range<usize> {
        self.len_start_offset().bits()..self.len_end_offset().bits()
    }
    #[allow(dead_code)]
    fn len_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.len_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn len_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn value_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.value_bit_range();
        match self.value() {
            Ok(value) => {
                let inner = value.field_tree().renamed("value").shifted(bit_range.start);
                let consumed = inner.bit_range.end.min(bit_range.end);
                let mut node = inner.with_bit_range(bit_range.clone());
                if let Ok(rest) = self.value_rest() && !rest.is_empty() {
                    node.children
                        .push(
                            ::binparse::FieldNode::new(
                                "rest",
                                "[u8]",
                                consumed..bit_range.end,
                                ::binparse::Value::Bytes(rest),
                            ),
                        );
                }
                node
            }
            Err(error) => {
                ::binparse::FieldNode::new(
                        "value",
                        "Inner",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Error(error))
            }
        }
    }
    pub fn value_rest(&self) -> ::binparse::ParseResult<&'a [u8]> {
        let start = 2usize;
        let end = start.saturating_add(self.len() as usize);
        Inner::parse(&self.data[start..end]).map(|(_, rest)| rest)
    }
    #[allow(clippy::identity_op)]
    pub fn value(&self) -> ::binparse::ParseResult<Inner<'a>> {
        let start = 2usize;
        let end = start.saturating_add(self.len() as usize);
        Inner::parse(&self.data[start..end]).map(|(value, _)| value)
    }
    pub fn value_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
            + ({
                ::binparse::Len {
                    byte: self.len() as usize,
                    bit: 0,
                }
            })
    }
    pub fn value_start_offset(&self) -> binparse::Len {
        self.len_end_offset()
    }
    pub fn value_bit_range(&self) -> ::core::ops::Range<usize> {
        self.value_start_offset().bits()..self.value_end_offset().bits()
    }
    #[allow(dead_code)]
    fn value_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.value_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn value_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn after_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.after_bit_range();
        ::binparse::FieldNode::new(
            "after",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.after())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn after(&self) -> u8 {
        {
            let offset = ::binparse::Len {
                byte: 2usize,
                bit: 0usize,
            }
                + ({
                    ::binparse::Len {
                        byte: self.len() as usize,
                        bit: 0,
                    }
                });
            debug_assert!(offset.bit == 0, "primitive requires byte alignment");
            self.data[offset.byte]
        }
    }
    pub fn after_end_offset(&self) -> binparse::Len {
        ({
            ::binparse::Len {
                byte: 2usize,
                bit: 0usize,
            }
                + ({
                    ::binparse::Len {
                        byte: self.len() as usize,
                        bit: 0,
                    }
                })
        })
            + ::binparse::Len {
                byte: 1usize,
                bit: 0usize,
            }
    }
    pub fn after_start_offset(&self) -> binparse::Len {
        self.value_end_offset()
    }
    pub fn after_bit_range(&self) -> ::core::ops::Range<usize> {
        self.after_start_offset().bits()..self.after_end_offset().bits()
    }
    #[allow(dead_code)]
    fn after_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.after_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn after_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Tlv<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Tlv::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Tlv::handoff(self)
    }
}
pub struct Inner<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Inner<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.a_fatal_check()?;
        me.a_recoverable_check()?;
        me.b_fatal_check()?;
        me.b_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.a_present_node());
        }
        {
            let me = self;
            children.push(me.b_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Inner",
                "Inner",
                0usize..self.b_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.a_fatal_check() {
                Err(error) => {
                    let start = me.a_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "a",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.a_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.a_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "a",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.a_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.b_fatal_check() {
                Err(error) => {
                    let start = me.b_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "b",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.b_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.b_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "b",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.b_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Inner",
                "Inner",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn a_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.a_bit_range();
        ::binparse::FieldNode::new(
            "a",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.a())),
        )
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
    pub fn a_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn a_bit_range(&self) -> ::core::ops::Range<usize> {
        self.a_start_offset().bits()..self.a_end_offset().bits()
    }
    #[allow(dead_code)]
    fn a_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.a_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn a_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn b_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.b_bit_range();
        ::binparse::FieldNode::new(
            "b",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.b())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn b(&self) -> u16 {
        u16::from_be_bytes(self.data[1usize..3usize].try_into().unwrap())
    }
    pub fn b_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 3usize,
            bit: 0usize,
        }
    }
    pub fn b_start_offset(&self) -> binparse::Len {
        self.a_end_offset()
    }
    pub fn b_bit_range(&self) -> ::core::ops::Range<usize> {
        self.b_start_offset().bits()..self.b_end_offset().bits()
    }
    #[allow(dead_code)]
    fn b_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.b_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn b_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Inner<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Inner::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Inner::handoff(self)
    }
}

"#,
    );
}

#[test]
fn len_on_primitive_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @len(n) a: u8 }");
    assert!(
        err.to_string()
            .contains("@len can only be applied to struct ref, union, or unsized array fields")
    );
}

#[test]
fn len_on_bitfield_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @len(n) a: b<4> }");
    assert!(
        err.to_string()
            .contains("@len can only be applied to struct ref, union, or unsized array fields")
    );
}

#[test]
fn len_on_constant_field_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @len(n) magic = xff }");
    assert!(
        err.to_string()
            .contains("@len can only be applied to struct ref, union, or unsized array fields")
    );
}

#[test]
fn len_on_counted_array_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @len(n) a: [u8; n] }");
    assert!(
        err.to_string()
            .contains("@len cannot be applied to a counted or expression-sized array")
    );
}

#[test]
fn len_on_bitfield_array_is_rejected() {
    let err = generate_err("struct Foo { n: u8, @len(n) @greedy(unsafe_eof) a: [b<4>] }");
    assert!(
        err.to_string()
            .contains("@len cannot be applied to a bitfield-element array")
    );
}

#[test]
fn len_on_greedy_array_is_accepted() {
    let code = generate("struct Foo { n: u8, @len(n) @greedy(unsafe_eof) a: [u8] }");
    assert!(code.contains("fn a_rest"));
}

#[test]
fn len_on_until_array_is_accepted() {
    let code = generate("struct Foo { n: u8, @len(n) @until(x00) a: [u8] }");
    assert!(code.contains("fn a_rest"));
}

#[test]
fn len_on_union_is_accepted() {
    let code = generate(
        "struct Inner { a: u8 }
         struct Foo { t: u8, n: u8, @len(n) v: union(t) { 1 => A { i: Inner }, _ => B { } } }",
    );
    assert!(code.contains("fn v_rest"));
}

#[test]
fn len_wrong_arg_count_is_rejected() {
    let err = generate_err(
        "struct Inner { a: u8 }
         struct Foo { n: u8, @len(n, 2) value: Inner }",
    );
    assert!(
        err.to_string()
            .contains("@len requires exactly 1 argument(s), got 2")
    );
}

#[test]
fn len_bound_smaller_than_fixed_inner_is_rejected() {
    let err = generate_err(
        "struct Inner { a: u32 }
         struct Foo { @len(2) value: Inner }",
    );
    assert!(
        err.to_string()
            .contains("@len(2) is smaller than the referenced struct's fixed length of 4 bytes")
    );
}

#[test]
fn len_unknown_field_is_rejected() {
    let err = generate_err(
        "struct Inner { a: u8 }
         struct Foo { @len(nope) value: Inner }",
    );
    assert!(err.to_string().contains("references field 'nope'"));
}

#[test]
fn len_bound_equal_to_fixed_inner_is_accepted() {
    let code = generate(
        "struct Inner { a: u32 }
         struct Foo { @len(4) value: Inner }",
    );
    assert!(code.contains("fn value_rest"));
}

#[test]
fn struct_level_len_fill_to_bound_array_is_accepted() {
    let code = generate("@len(total_len) struct Foo { total_len: u16, payload: [u8] }");
    assert!(code.contains("fn struct_len"));
}

#[test]
fn bare_sizeless_array_without_struct_len_is_rejected() {
    let err = generate_err("struct Foo { total_len: u16, payload: [u8] }");
    assert!(
        err.to_string()
            .contains("array without size requires @until, @greedy, or @hook")
    );
}

#[test]
fn fill_to_bound_array_not_last_is_rejected() {
    let err = generate_err("@len(total_len) struct Foo { total_len: u16, payload: [u8], tail: u8 }");
    assert!(
        err.to_string()
            .contains("fill-to-bound array field 'payload' must be the last field in the struct")
    );
}

#[test]
fn struct_level_len_on_conditional_field_is_rejected() {
    let err = generate_err("@len(n) struct Foo { f: u8, if (f > 0) { n: u8 } payload: [u8] }");
    assert!(err.to_string().contains("references conditional field 'n'"));
}

#[test]
fn struct_level_len_wrong_arg_count_is_rejected() {
    let err = generate_err("@len(total_len, 2) struct Foo { total_len: u16, payload: [u8] }");
    assert!(
        err.to_string()
            .contains("@len requires exactly 1 argument(s), got 2")
    );
}

#[test]
fn golden_struct_level_len() {
    assert_generated_eq(
        "@len(len) struct Bounded { len: u8, value: u16 }",
        r#"
pub struct Bounded<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Bounded<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.len_fatal_check()?;
        me.len_recoverable_check()?;
        me.value_fatal_check()?;
        me.value_recoverable_check()?;
        let len = me.struct_len();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.len_present_node());
        }
        {
            let me = self;
            children.push(me.value_present_node());
        }
        {
            let bound_end = self.struct_len().byte.min(self.data.len());
            let consumed = children
                .last()
                .map(|child| child.bit_range.end.div_ceil(8))
                .unwrap_or(0)
                .min(bound_end);
            if consumed < bound_end {
                children
                    .push(
                        ::binparse::FieldNode::new(
                                "trailing",
                                "[u8]",
                                consumed.saturating_mul(8)..bound_end.saturating_mul(8),
                                ::binparse::Value::Bytes(&self.data[consumed..bound_end]),
                            )
                            .hide(),
                    );
            }
        }
        let mut root = ::binparse::FieldNode::new(
                "Bounded",
                "Bounded",
                0usize..self.struct_len().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.len_fatal_check() {
                Err(error) => {
                    let start = me.len_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "len",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.len_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.len_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "len",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.len_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.value_fatal_check() {
                Err(error) => {
                    let start = me.value_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "value",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.value_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.value_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "value",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.value_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            let bound = me.struct_len().byte;
            if data.len() < bound {
                fatal = Some(binparse::ParseError::NotEnoughData {
                    expected: bound,
                    got: data.len(),
                });
            } else {
                let consumed = children
                    .last()
                    .map(|child| child.bit_range.end.div_ceil(8))
                    .unwrap_or(0)
                    .min(bound);
                if consumed < bound {
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "trailing",
                                    "[u8]",
                                    consumed.saturating_mul(8)..bound.saturating_mul(8),
                                    ::binparse::Value::Bytes(&me.data[consumed..bound]),
                                )
                                .hide(),
                        );
                }
            }
        }
        let root_end = if fatal.is_some() {
            children.last().map(|child| child.bit_range.end).unwrap_or(0)
        } else {
            me.struct_len().bits()
        };
        let mut root = ::binparse::FieldNode::new(
                "Bounded",
                "Bounded",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn len_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.len_bit_range();
        ::binparse::FieldNode::new(
            "len",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.len())),
        )
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
    pub fn len_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn len_bit_range(&self) -> ::core::ops::Range<usize> {
        self.len_start_offset().bits()..self.len_end_offset().bits()
    }
    #[allow(dead_code)]
    fn len_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.len_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn len_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn value_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.value_bit_range();
        ::binparse::FieldNode::new(
            "value",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.value())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn value(&self) -> u16 {
        u16::from_be_bytes(self.data[1usize..3usize].try_into().unwrap())
    }
    pub fn value_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 3usize,
            bit: 0usize,
        }
    }
    pub fn value_start_offset(&self) -> binparse::Len {
        self.len_end_offset()
    }
    pub fn value_bit_range(&self) -> ::core::ops::Range<usize> {
        self.value_start_offset().bits()..self.value_end_offset().bits()
    }
    #[allow(dead_code)]
    fn value_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.value_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn value_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn struct_len(&self) -> binparse::Len {
        binparse::Len {
            byte: self.len() as usize,
            bit: 0,
        }
    }
}
impl<'a> ::binparse::Dissect<'a> for Bounded<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Bounded::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Bounded::handoff(self)
    }
}
"#,
    );
}

#[test]
fn golden_struct_level_len_fill_to_bound() {
    assert_generated_eq(
        "@len(total_len) struct Filled { total_len: u16, payload: [u8] }",
        r#"
#[allow(non_camel_case_types)]
pub struct Filled_payload_Iterator<'a> {
    idx: usize,
    count: usize,
    data: &'a [u8],
}
impl<'a> ::std::iter::Iterator for Filled_payload_Iterator<'a> {
    type Item = ::binparse::ParseResult<u8>;
    fn next(&mut self) -> std::option::Option<Self::Item> {
        if self.idx == self.count {
            return None;
        }
        self.idx += 1;
        let value = self.data[0];
        self.data = &self.data[1..];
        Some(Ok(value))
    }
}
pub struct Filled<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Filled<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.total_len_fatal_check()?;
        me.total_len_recoverable_check()?;
        me.payload_fatal_check()?;
        me.payload_recoverable_check()?;
        let len = me.struct_len();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.total_len_present_node());
        }
        {
            let me = self;
            children.push(me.payload_present_node());
        }
        {
            let bound_end = self.struct_len().byte.min(self.data.len());
            let consumed = children
                .last()
                .map(|child| child.bit_range.end.div_ceil(8))
                .unwrap_or(0)
                .min(bound_end);
            if consumed < bound_end {
                children
                    .push(
                        ::binparse::FieldNode::new(
                                "trailing",
                                "[u8]",
                                consumed.saturating_mul(8)..bound_end.saturating_mul(8),
                                ::binparse::Value::Bytes(&self.data[consumed..bound_end]),
                            )
                            .hide(),
                    );
            }
        }
        let mut root = ::binparse::FieldNode::new(
                "Filled",
                "Filled",
                0usize..self.struct_len().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.total_len_fatal_check() {
                Err(error) => {
                    let start = me.total_len_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "total_len",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.total_len_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.total_len_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "total_len",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.total_len_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.payload_fatal_check() {
                Err(error) => {
                    let start = me.payload_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "payload",
                                    "[u8]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.payload_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.payload_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "payload",
                                            "[u8]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.payload_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            let bound = me.struct_len().byte;
            if data.len() < bound {
                fatal = Some(binparse::ParseError::NotEnoughData {
                    expected: bound,
                    got: data.len(),
                });
            } else {
                let consumed = children
                    .last()
                    .map(|child| child.bit_range.end.div_ceil(8))
                    .unwrap_or(0)
                    .min(bound);
                if consumed < bound {
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "trailing",
                                    "[u8]",
                                    consumed.saturating_mul(8)..bound.saturating_mul(8),
                                    ::binparse::Value::Bytes(&me.data[consumed..bound]),
                                )
                                .hide(),
                        );
                }
            }
        }
        let root_end = if fatal.is_some() {
            children.last().map(|child| child.bit_range.end).unwrap_or(0)
        } else {
            me.struct_len().bits()
        };
        let mut root = ::binparse::FieldNode::new(
                "Filled",
                "Filled",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn total_len_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.total_len_bit_range();
        ::binparse::FieldNode::new(
            "total_len",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.total_len())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn total_len(&self) -> u16 {
        u16::from_be_bytes(self.data[0usize..2usize].try_into().unwrap())
    }
    pub fn total_len_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
    }
    pub fn total_len_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn total_len_bit_range(&self) -> ::core::ops::Range<usize> {
        self.total_len_start_offset().bits()..self.total_len_end_offset().bits()
    }
    #[allow(dead_code)]
    fn total_len_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.total_len_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn total_len_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn payload_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.payload_bit_range();
        {
            let mut elem_nodes = ::std::vec::Vec::new();
            if let Ok(iter) = self.payload() {
                let mut start = bit_range.start;
                for (i, elem) in iter.enumerate() {
                    let end = start.saturating_add(8usize);
                    match elem {
                        Ok(value) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                        i.to_string(),
                                        "u8",
                                        start..end,
                                        ::binparse::Value::UInt(u128::from(value)),
                                    ),
                                )
                        }
                        Err(error) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                            i.to_string(),
                                            "u8",
                                            start..start,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                    }
                    start = end;
                }
            }
            ::binparse::FieldNode::new(
                    "payload",
                    "[u8]",
                    bit_range.clone(),
                    ::binparse::Value::Array,
                )
                .with_children(elem_nodes)
        }
    }
    #[allow(clippy::identity_op)]
    pub fn payload(&self) -> ::binparse::ParseResult<Filled_payload_Iterator<'a>> {
        Ok(Filled_payload_Iterator {
            idx: 0,
            count: self
                .data[..((self.total_len() as usize).min(self.data.len()))]
                .len()
                .saturating_sub(2usize),
            data: &self
                .data[..((self.total_len() as usize).min(self.data.len()))][2usize..],
        })
    }
    pub fn payload_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
            + ({
                {
                    let start = 2usize;
                    ::binparse::Len {
                        byte: self
                            .data[..((self.total_len() as usize).min(self.data.len()))]
                            .len()
                            .saturating_sub(start),
                        bit: 0,
                    }
                }
            })
    }
    pub fn payload_start_offset(&self) -> binparse::Len {
        self.total_len_end_offset()
    }
    pub fn payload_bit_range(&self) -> ::core::ops::Range<usize> {
        self.payload_start_offset().bits()..self.payload_end_offset().bits()
    }
    #[allow(dead_code)]
    fn payload_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.payload_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn payload_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn struct_len(&self) -> binparse::Len {
        binparse::Len {
            byte: self.total_len() as usize,
            bit: 0,
        }
    }
}
impl<'a> ::binparse::Dissect<'a> for Filled<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Filled::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Filled::handoff(self)
    }
}
"#,
    );
}


#[test]
fn golden_len_bounded_union() {
    assert_generated_eq(
        r#"
struct Inner { a: u8, b: u16 }
struct Tlv { tag: u8, len: u8, @len(len) value: union(tag) { 1 => Addr { inner: Inner }, _ => Raw { @greedy(unsafe_eof) bytes: [u8] } }, after: u8 }
"#,
        r#"
pub struct Inner<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Inner<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.a_fatal_check()?;
        me.a_recoverable_check()?;
        me.b_fatal_check()?;
        me.b_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.a_present_node());
        }
        {
            let me = self;
            children.push(me.b_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Inner",
                "Inner",
                0usize..self.b_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.a_fatal_check() {
                Err(error) => {
                    let start = me.a_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "a",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.a_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.a_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "a",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.a_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.b_fatal_check() {
                Err(error) => {
                    let start = me.b_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "b",
                                    "u16",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.b_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.b_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "b",
                                            "u16",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.b_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Inner",
                "Inner",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn a_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.a_bit_range();
        ::binparse::FieldNode::new(
            "a",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.a())),
        )
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
    pub fn a_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn a_bit_range(&self) -> ::core::ops::Range<usize> {
        self.a_start_offset().bits()..self.a_end_offset().bits()
    }
    #[allow(dead_code)]
    fn a_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.a_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn a_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn b_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.b_bit_range();
        ::binparse::FieldNode::new(
            "b",
            "u16",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.b())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn b(&self) -> u16 {
        u16::from_be_bytes(self.data[1usize..3usize].try_into().unwrap())
    }
    pub fn b_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 3usize,
            bit: 0usize,
        }
    }
    pub fn b_start_offset(&self) -> binparse::Len {
        self.a_end_offset()
    }
    pub fn b_bit_range(&self) -> ::core::ops::Range<usize> {
        self.b_start_offset().bits()..self.b_end_offset().bits()
    }
    #[allow(dead_code)]
    fn b_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.b_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn b_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Inner<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Inner::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Inner::handoff(self)
    }
}
#[allow(non_camel_case_types)]
pub struct Tlv_value_Addr<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Tlv_value_Addr<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.inner_fatal_check()?;
        me.inner_recoverable_check()?;
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.inner_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Tlv_value_Addr",
                "Tlv_value_Addr",
                0usize..self.inner_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.inner_fatal_check() {
                Err(error) => {
                    let start = me.inner_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "inner",
                                    "Inner",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.inner_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.inner_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "inner",
                                            "Inner",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.inner_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Tlv_value_Addr",
                "Tlv_value_Addr",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn inner_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.inner_bit_range();
        match self.inner() {
            Ok(value) => value.field_tree().renamed("inner").shifted(bit_range.start),
            Err(error) => {
                ::binparse::FieldNode::new(
                        "inner",
                        "Inner",
                        bit_range.clone(),
                        ::binparse::Value::Opaque,
                    )
                    .with_status(::binparse::Status::Error(error))
            }
        }
    }
    #[allow(clippy::identity_op)]
    pub fn inner(&self) -> ::binparse::ParseResult<Inner<'a>> {
        Inner::parse(&self.data[0usize..]).map(|(value, _)| value)
    }
    pub fn inner_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 3usize,
            bit: 0usize,
        }
    }
    pub fn inner_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn inner_bit_range(&self) -> ::core::ops::Range<usize> {
        self.inner_start_offset().bits()..self.inner_end_offset().bits()
    }
    #[allow(dead_code)]
    fn inner_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.inner_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn inner_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Tlv_value_Addr<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Tlv_value_Addr::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Tlv_value_Addr::handoff(self)
    }
}
#[allow(non_camel_case_types)]
pub struct Tlv_value_Raw_bytes_Iterator<'a> {
    idx: usize,
    count: usize,
    data: &'a [u8],
}
impl<'a> ::std::iter::Iterator for Tlv_value_Raw_bytes_Iterator<'a> {
    type Item = ::binparse::ParseResult<u8>;
    fn next(&mut self) -> std::option::Option<Self::Item> {
        if self.idx == self.count {
            return None;
        }
        self.idx += 1;
        let value = self.data[0];
        self.data = &self.data[1..];
        Some(Ok(value))
    }
}
#[allow(non_camel_case_types)]
pub struct Tlv_value_Raw<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Tlv_value_Raw<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.bytes_fatal_check()?;
        me.bytes_recoverable_check()?;
        let len = me.bytes_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.bytes_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Tlv_value_Raw",
                "Tlv_value_Raw",
                0usize..self.bytes_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.bytes_fatal_check() {
                Err(error) => {
                    let start = me.bytes_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "bytes",
                                    "[u8]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.bytes_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.bytes_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "bytes",
                                            "[u8]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.bytes_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Tlv_value_Raw",
                "Tlv_value_Raw",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn bytes_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.bytes_bit_range();
        {
            let mut elem_nodes = ::std::vec::Vec::new();
            if let Ok(iter) = self.bytes() {
                let mut start = bit_range.start;
                for (i, elem) in iter.enumerate() {
                    let end = start.saturating_add(8usize);
                    match elem {
                        Ok(value) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                        i.to_string(),
                                        "u8",
                                        start..end,
                                        ::binparse::Value::UInt(u128::from(value)),
                                    ),
                                )
                        }
                        Err(error) => {
                            elem_nodes
                                .push(
                                    ::binparse::FieldNode::new(
                                            i.to_string(),
                                            "u8",
                                            start..start,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                    }
                    start = end;
                }
            }
            ::binparse::FieldNode::new(
                    "bytes",
                    "[u8]",
                    bit_range.clone(),
                    ::binparse::Value::Array,
                )
                .with_children(elem_nodes)
        }
    }
    #[allow(clippy::identity_op)]
    pub fn bytes(&self) -> ::binparse::ParseResult<Tlv_value_Raw_bytes_Iterator<'a>> {
        Ok(Tlv_value_Raw_bytes_Iterator {
            idx: 0,
            count: self.data.len().saturating_sub(0usize),
            data: &self.data[0usize..],
        })
    }
    pub fn bytes_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 0usize,
            bit: 0usize,
        }
            + ({
                {
                    let start = 0usize;
                    ::binparse::Len {
                        byte: self.data.len().saturating_sub(start),
                        bit: 0,
                    }
                }
            })
    }
    pub fn bytes_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn bytes_bit_range(&self) -> ::core::ops::Range<usize> {
        self.bytes_start_offset().bits()..self.bytes_end_offset().bits()
    }
    #[allow(dead_code)]
    fn bytes_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.bytes_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn bytes_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Tlv_value_Raw<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Tlv_value_Raw::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Tlv_value_Raw::handoff(self)
    }
}
#[allow(non_camel_case_types)]
pub enum Tlv_value<'a> {
    Addr(Tlv_value_Addr<'a>),
    Raw(Tlv_value_Raw<'a>),
}
pub struct Tlv<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Tlv<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.tag_fatal_check()?;
        me.tag_recoverable_check()?;
        me.len_fatal_check()?;
        me.len_recoverable_check()?;
        me.value_fatal_check()?;
        me.value_recoverable_check()?;
        me.after_fatal_check()?;
        me.after_recoverable_check()?;
        let len = me.after_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.tag_present_node());
        }
        {
            let me = self;
            children.push(me.len_present_node());
        }
        {
            let me = self;
            children.push(me.value_present_node());
        }
        {
            let me = self;
            children.push(me.after_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Tlv",
                "Tlv",
                0usize..self.after_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.tag_fatal_check() {
                Err(error) => {
                    let start = me.tag_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "tag",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.tag_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.tag_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "tag",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.tag_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.len_fatal_check() {
                Err(error) => {
                    let start = me.len_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "len",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.len_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.len_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "len",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.len_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.value_fatal_check() {
                Err(error) => {
                    let start = me.value_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "value",
                                    "union",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.value_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.value_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "value",
                                            "union",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.value_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.after_fatal_check() {
                Err(error) => {
                    let start = me.after_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "after",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.after_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.after_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "after",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.after_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Tlv",
                "Tlv",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn tag_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.tag_bit_range();
        ::binparse::FieldNode::new(
            "tag",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.tag())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn tag(&self) -> u8 {
        self.data[0usize]
    }
    pub fn tag_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
    }
    pub fn tag_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn tag_bit_range(&self) -> ::core::ops::Range<usize> {
        self.tag_start_offset().bits()..self.tag_end_offset().bits()
    }
    #[allow(dead_code)]
    fn tag_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.tag_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn tag_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn len_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.len_bit_range();
        ::binparse::FieldNode::new(
            "len",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.len())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn len(&self) -> u8 {
        self.data[1usize]
    }
    pub fn len_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
    }
    pub fn len_start_offset(&self) -> binparse::Len {
        self.tag_end_offset()
    }
    pub fn len_bit_range(&self) -> ::core::ops::Range<usize> {
        self.len_start_offset().bits()..self.len_end_offset().bits()
    }
    #[allow(dead_code)]
    fn len_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.len_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn len_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn value_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.value_bit_range();
        {
            let mut node = match self.value() {
                Ok(Tlv_value::Addr(value)) => {
                    let inner = value
                        .field_tree()
                        .renamed("Addr")
                        .shifted(bit_range.start);
                    ::binparse::FieldNode::new(
                            "value",
                            "union",
                            bit_range.clone(),
                            ::binparse::Value::UnionVariant("Addr"),
                        )
                        .with_children(::std::vec![inner])
                }
                Ok(Tlv_value::Raw(value)) => {
                    let inner = value
                        .field_tree()
                        .renamed("Raw")
                        .shifted(bit_range.start);
                    ::binparse::FieldNode::new(
                            "value",
                            "union",
                            bit_range.clone(),
                            ::binparse::Value::UnionVariant("Raw"),
                        )
                        .with_children(::std::vec![inner])
                }
                Err(error) => {
                    ::binparse::FieldNode::new(
                            "value",
                            "union",
                            bit_range.clone(),
                            ::binparse::Value::Opaque,
                        )
                        .with_status(::binparse::Status::Error(error))
                }
            };
            if let Ok(rest) = self.value_rest() && !rest.is_empty() {
                let consumed = node
                    .children
                    .last()
                    .map(|child| child.bit_range.end)
                    .unwrap_or(bit_range.start)
                    .min(bit_range.end);
                node.children
                    .push(
                        ::binparse::FieldNode::new(
                            "rest",
                            "[u8]",
                            consumed..bit_range.end,
                            ::binparse::Value::Bytes(rest),
                        ),
                    );
            }
            node
        }
    }
    fn value_union_check(&self) -> Result<(), binparse::ParseError> {
        match self.tag() as usize {
            1 => {
                Tlv_value_Addr::parse(
                    &self
                        .data[(2usize)
                        .min(
                            self.data.len(),
                        )..(({ 2usize })
                        .saturating_add(self.len() as usize)
                        .min(self.data.len()))],
                )?;
            }
            _ => {
                Tlv_value_Raw::parse(
                    &self
                        .data[(2usize)
                        .min(
                            self.data.len(),
                        )..(({ 2usize })
                        .saturating_add(self.len() as usize)
                        .min(self.data.len()))],
                )?;
            }
        }
        Ok(())
    }
    pub fn value_rest(&self) -> ::binparse::ParseResult<&'a [u8]> {
        match self.tag() as usize {
            1 => {
                Tlv_value_Addr::parse(
                        &self
                            .data[2usize..(({ 2usize })
                            .saturating_add(self.len() as usize)
                            .min(self.data.len()))],
                    )
                    .map(|(_, rest)| rest)
            }
            _ => {
                Tlv_value_Raw::parse(
                        &self
                            .data[2usize..(({ 2usize })
                            .saturating_add(self.len() as usize)
                            .min(self.data.len()))],
                    )
                    .map(|(_, rest)| rest)
            }
        }
    }
    #[allow(clippy::identity_op)]
    pub fn value(&self) -> ::binparse::ParseResult<Tlv_value<'a>> {
        match self.tag() as usize {
            1 => {
                Tlv_value_Addr::parse(
                        &self
                            .data[2usize..(({ 2usize })
                            .saturating_add(self.len() as usize)
                            .min(self.data.len()))],
                    )
                    .map(|(value, _)| Tlv_value::Addr(value))
            }
            _ => {
                Tlv_value_Raw::parse(
                        &self
                            .data[2usize..(({ 2usize })
                            .saturating_add(self.len() as usize)
                            .min(self.data.len()))],
                    )
                    .map(|(value, _)| Tlv_value::Raw(value))
            }
        }
    }
    pub fn value_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
            + ({
                ::binparse::Len {
                    byte: self.len() as usize,
                    bit: 0,
                }
            })
    }
    pub fn value_start_offset(&self) -> binparse::Len {
        self.len_end_offset()
    }
    pub fn value_bit_range(&self) -> ::core::ops::Range<usize> {
        self.value_start_offset().bits()..self.value_end_offset().bits()
    }
    #[allow(dead_code)]
    fn value_fatal_check(&self) -> Result<(), binparse::ParseError> {
        self.value_union_check()?;
        {
            let len = self.value_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn value_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn after_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.after_bit_range();
        ::binparse::FieldNode::new(
            "after",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.after())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn after(&self) -> u8 {
        {
            let offset = ::binparse::Len {
                byte: 2usize,
                bit: 0usize,
            }
                + ({
                    ::binparse::Len {
                        byte: self.len() as usize,
                        bit: 0,
                    }
                });
            debug_assert!(offset.bit == 0, "primitive requires byte alignment");
            self.data[offset.byte]
        }
    }
    pub fn after_end_offset(&self) -> binparse::Len {
        ({
            ::binparse::Len {
                byte: 2usize,
                bit: 0usize,
            }
                + ({
                    ::binparse::Len {
                        byte: self.len() as usize,
                        bit: 0,
                    }
                })
        })
            + ::binparse::Len {
                byte: 1usize,
                bit: 0usize,
            }
    }
    pub fn after_start_offset(&self) -> binparse::Len {
        self.value_end_offset()
    }
    pub fn after_bit_range(&self) -> ::core::ops::Range<usize> {
        self.after_start_offset().bits()..self.after_end_offset().bits()
    }
    #[allow(dead_code)]
    fn after_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.after_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn after_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Tlv<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Tlv::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Tlv::handoff(self)
    }
}
"#,
    );
}

#[test]
fn golden_len_bounded_greedy_array() {
    assert_generated_eq(
        r#"
struct Frame { tag: u8, len: u8, @len(len) @greedy(unsafe_eof) body: [u8], after: u8 }
"#,
        r#"
#[allow(non_camel_case_types)]
pub struct Frame_body_Iterator<'a> {
    idx: usize,
    count: usize,
    data: &'a [u8],
}
impl<'a> ::std::iter::Iterator for Frame_body_Iterator<'a> {
    type Item = ::binparse::ParseResult<u8>;
    fn next(&mut self) -> std::option::Option<Self::Item> {
        if self.idx == self.count {
            return None;
        }
        self.idx += 1;
        let value = self.data[0];
        self.data = &self.data[1..];
        Some(Ok(value))
    }
}
pub struct Frame<'a> {
    #[allow(dead_code)]
    data: &'a [u8],
}
impl<'a> Frame<'a> {
    pub fn parse(data: &'a [u8]) -> Result<(Self, &'a [u8]), binparse::ParseError> {
        let me = Self { data };
        me.tag_fatal_check()?;
        me.tag_recoverable_check()?;
        me.len_fatal_check()?;
        me.len_recoverable_check()?;
        me.body_fatal_check()?;
        me.body_recoverable_check()?;
        me.after_fatal_check()?;
        me.after_recoverable_check()?;
        let len = me.after_end_offset();
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
    pub fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        let mut children = ::std::vec::Vec::new();
        {
            let me = self;
            children.push(me.tag_present_node());
        }
        {
            let me = self;
            children.push(me.len_present_node());
        }
        {
            let me = self;
            children.push(me.body_present_node());
        }
        {
            let me = self;
            children.push(me.after_present_node());
        }
        let mut root = ::binparse::FieldNode::new(
                "Frame",
                "Frame",
                0usize..self.after_end_offset().bits(),
                ::binparse::Value::Struct,
            )
            .with_children(children);
        root.set_paths("");
        root
    }
    pub fn dissect(data: &'a [u8]) -> ::binparse::FieldNode<'a> {
        let me = Self { data };
        let mut children: ::std::vec::Vec<::binparse::FieldNode<'a>> = ::std::vec::Vec::new();
        let mut fatal: Option<::binparse::ParseError> = None;
        if fatal.is_none() {
            match me.tag_fatal_check() {
                Err(error) => {
                    let start = me.tag_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "tag",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.tag_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.tag_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "tag",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.tag_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.len_fatal_check() {
                Err(error) => {
                    let start = me.len_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "len",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.len_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.len_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "len",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.len_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.body_fatal_check() {
                Err(error) => {
                    let start = me.body_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "body",
                                    "[u8]",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.body_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.body_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "body",
                                            "[u8]",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.body_present_node());
                        }
                    }
                }
            }
        }
        if fatal.is_none() {
            match me.after_fatal_check() {
                Err(error) => {
                    let start = me.after_start_offset().bits();
                    children
                        .push(
                            ::binparse::FieldNode::new(
                                    "after",
                                    "u8",
                                    start..start,
                                    ::binparse::Value::Opaque,
                                )
                                .with_status(::binparse::Status::Error(error)),
                        );
                    fatal = Some(error);
                }
                Ok(()) => {
                    match me.after_recoverable_check() {
                        Err(error) => {
                            let bit_range = me.after_bit_range();
                            children
                                .push(
                                    ::binparse::FieldNode::new(
                                            "after",
                                            "u8",
                                            bit_range,
                                            ::binparse::Value::Opaque,
                                        )
                                        .with_status(::binparse::Status::Error(error)),
                                );
                        }
                        Ok(()) => {
                            children.push(me.after_present_node());
                        }
                    }
                }
            }
        }
        let root_end = children.last().map(|child| child.bit_range.end).unwrap_or(0);
        let mut root = ::binparse::FieldNode::new(
                "Frame",
                "Frame",
                0usize..root_end,
                ::binparse::Value::Struct,
            )
            .with_children(children);
        if let Some(error) = fatal {
            root = root.with_status(::binparse::Status::Error(error));
        }
        root.set_paths("");
        root
    }
    pub fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        None
    }
    #[allow(dead_code)]
    fn tag_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.tag_bit_range();
        ::binparse::FieldNode::new(
            "tag",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.tag())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn tag(&self) -> u8 {
        self.data[0usize]
    }
    pub fn tag_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 1usize,
            bit: 0usize,
        }
    }
    pub fn tag_start_offset(&self) -> binparse::Len {
        binparse::Len::ZERO
    }
    pub fn tag_bit_range(&self) -> ::core::ops::Range<usize> {
        self.tag_start_offset().bits()..self.tag_end_offset().bits()
    }
    #[allow(dead_code)]
    fn tag_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.tag_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn tag_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn len_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.len_bit_range();
        ::binparse::FieldNode::new(
            "len",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.len())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn len(&self) -> u8 {
        self.data[1usize]
    }
    pub fn len_end_offset(&self) -> binparse::Len {
        binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
    }
    pub fn len_start_offset(&self) -> binparse::Len {
        self.tag_end_offset()
    }
    pub fn len_bit_range(&self) -> ::core::ops::Range<usize> {
        self.len_start_offset().bits()..self.len_end_offset().bits()
    }
    #[allow(dead_code)]
    fn len_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.len_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn len_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn body_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.body_bit_range();
        {
            let mut node = {
                let mut elem_nodes = ::std::vec::Vec::new();
                if let Ok(iter) = self.body() {
                    let mut start = bit_range.start;
                    for (i, elem) in iter.enumerate() {
                        let end = start.saturating_add(8usize);
                        match elem {
                            Ok(value) => {
                                elem_nodes
                                    .push(
                                        ::binparse::FieldNode::new(
                                            i.to_string(),
                                            "u8",
                                            start..end,
                                            ::binparse::Value::UInt(u128::from(value)),
                                        ),
                                    )
                            }
                            Err(error) => {
                                elem_nodes
                                    .push(
                                        ::binparse::FieldNode::new(
                                                i.to_string(),
                                                "u8",
                                                start..start,
                                                ::binparse::Value::Opaque,
                                            )
                                            .with_status(::binparse::Status::Error(error)),
                                    );
                            }
                        }
                        start = end;
                    }
                }
                ::binparse::FieldNode::new(
                        "body",
                        "[u8]",
                        bit_range.clone(),
                        ::binparse::Value::Array,
                    )
                    .with_children(elem_nodes)
            };
            if let Ok(rest) = self.body_rest() && !rest.is_empty() {
                let consumed = node
                    .children
                    .last()
                    .map(|child| child.bit_range.end)
                    .unwrap_or(bit_range.start)
                    .min(bit_range.end);
                node.children
                    .push(
                        ::binparse::FieldNode::new(
                            "rest",
                            "[u8]",
                            consumed..bit_range.end,
                            ::binparse::Value::Bytes(rest),
                        ),
                    );
            }
            node
        }
    }
    pub fn body_rest(&self) -> ::binparse::ParseResult<&'a [u8]> {
        let end = ({ 2usize }).saturating_add(self.len() as usize).min(self.data.len());
        let consumed = (2usize)
            .saturating_add(
                ({
                    {
                        let start = 2usize;
                        ::binparse::Len {
                            byte: self
                                .data[..(({ 2usize })
                                    .saturating_add(self.len() as usize)
                                    .min(self.data.len()))]
                                .len()
                                .saturating_sub(start),
                            bit: 0,
                        }
                    }
                })
                    .byte_ceil(),
            );
        if consumed > end {
            return Err(::binparse::ParseError::NotEnoughData {
                expected: consumed,
                got: end,
            });
        }
        Ok(&self.data[consumed..end])
    }
    #[allow(clippy::identity_op)]
    pub fn body(&self) -> ::binparse::ParseResult<Frame_body_Iterator<'a>> {
        Ok(Frame_body_Iterator {
            idx: 0,
            count: self
                .data[..(({ 2usize })
                    .saturating_add(self.len() as usize)
                    .min(self.data.len()))]
                .len()
                .saturating_sub(2usize),
            data: &self
                .data[..(({ 2usize })
                .saturating_add(self.len() as usize)
                .min(self.data.len()))][2usize..],
        })
    }
    pub fn body_end_offset(&self) -> binparse::Len {
        ::binparse::Len {
            byte: 2usize,
            bit: 0usize,
        }
            + ({
                ::binparse::Len {
                    byte: self.len() as usize,
                    bit: 0,
                }
            })
    }
    pub fn body_start_offset(&self) -> binparse::Len {
        self.len_end_offset()
    }
    pub fn body_bit_range(&self) -> ::core::ops::Range<usize> {
        self.body_start_offset().bits()..self.body_end_offset().bits()
    }
    #[allow(dead_code)]
    fn body_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.body_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn body_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
    #[allow(dead_code)]
    fn after_present_node(&self) -> ::binparse::FieldNode<'a> {
        let bit_range = self.after_bit_range();
        ::binparse::FieldNode::new(
            "after",
            "u8",
            bit_range.clone(),
            ::binparse::Value::UInt(u128::from(self.after())),
        )
    }
    #[allow(clippy::identity_op)]
    pub fn after(&self) -> u8 {
        {
            let offset = ::binparse::Len {
                byte: 2usize,
                bit: 0usize,
            }
                + ({
                    ::binparse::Len {
                        byte: self.len() as usize,
                        bit: 0,
                    }
                });
            debug_assert!(offset.bit == 0, "primitive requires byte alignment");
            self.data[offset.byte]
        }
    }
    pub fn after_end_offset(&self) -> binparse::Len {
        ({
            ::binparse::Len {
                byte: 2usize,
                bit: 0usize,
            }
                + ({
                    ::binparse::Len {
                        byte: self.len() as usize,
                        bit: 0,
                    }
                })
        })
            + ::binparse::Len {
                byte: 1usize,
                bit: 0usize,
            }
    }
    pub fn after_start_offset(&self) -> binparse::Len {
        self.body_end_offset()
    }
    pub fn after_bit_range(&self) -> ::core::ops::Range<usize> {
        self.after_start_offset().bits()..self.after_end_offset().bits()
    }
    #[allow(dead_code)]
    fn after_fatal_check(&self) -> Result<(), binparse::ParseError> {
        {
            let len = self.after_end_offset();
            let expected = len.byte_ceil();
            if self.data.len() < expected {
                return Err(binparse::ParseError::NotEnoughData {
                    expected,
                    got: self.data.len(),
                });
            }
        }
        Ok(())
    }
    #[allow(dead_code)]
    fn after_recoverable_check(&self) -> Result<(), binparse::ParseError> {
        Ok(())
    }
}
impl<'a> ::binparse::Dissect<'a> for Frame<'a> {
    fn field_tree(&self) -> ::binparse::FieldNode<'a> {
        Frame::field_tree(self)
    }
    fn handoff(&self) -> Option<::binparse::Handoff<'a>> {
        Frame::handoff(self)
    }
}
"#,
    );
}
