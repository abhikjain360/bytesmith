use bytesmith_dsl as ast;
use proc_macro2::TokenStream;
use quote::format_ident;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Endian {
    #[default]
    Big,
    Little,
}

/// Bit order for bitfields within a byte.
///
/// The default is `Msb`: the first declared bitfield occupies the most
/// significant bits of its byte, matching network protocol diagrams (e.g. the
/// IPv4 `version` nibble is the high nibble of byte 0). `@bit_order(lsb)`
/// makes the first declared bitfield occupy the least significant bits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum BitOrder {
    #[default]
    Msb,
    Lsb,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Inherited {
    pub endian: Endian,
    pub bit_order: BitOrder,
}

#[derive(Debug, Clone)]
pub(crate) struct Hook {
    pub fn_path: TokenStream,
    pub return_ty: TokenStream,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("@{attr} requires exactly {expected} argument(s), got {got}")]
    WrongArgCount {
        attr: &'static str,
        expected: usize,
        got: usize,
    },
    #[error("@endian argument must be 'big' or 'little', got '{0}'")]
    InvalidEndianValue(String),
    #[error("@endian cannot be applied to single-byte integers (no endianness)")]
    EndianOnSingleByte,
    #[error("@endian cannot be applied to bitfields")]
    EndianOnBitfield,
    #[error(
        "@endian cannot be applied to struct ref (struct uses its own definition's endianness)"
    )]
    EndianOnStructRef,
    #[error("@bit_order argument must be 'msb' or 'lsb', got '{0}'")]
    InvalidBitOrderValue(String),
    #[error("@bit_order can only be applied to bitfields")]
    BitOrderOnNonBitfield,
    #[error("@hook arguments must be paths (fn_name, ReturnType)")]
    InvalidHookArg,
    #[error("@hook on VLA requires [u8] type")]
    HookVlaNotU8,
    #[error("@check and @range can only be applied to primitive and bitfield fields")]
    ValidationOnNonNumeric,
    #[error("@greedy argument must be 'unsafe_eof', got '{0}'")]
    InvalidGreedyValue(String),
    #[error("@until sentinel must be an integer literal fitting in one byte")]
    InvalidUntilSentinel,
    #[error("@{0} can only be applied to array fields")]
    ArrayAttrOnNonArray(&'static str),
    #[error("@{0} cannot be combined with @hook")]
    ArrayAttrWithHook(&'static str),
    #[error("@{0} requires an array without an explicit size")]
    ArrayAttrOnSizedArray(&'static str),
    #[error("@until and @greedy cannot be combined")]
    UntilWithGreedy,
    #[error("@greedy with dynamic-length elements requires @max_iter")]
    GreedyRequiresMaxIter,
    #[error("@{0} argument must be a positive integer literal")]
    InvalidPaddingArg(&'static str),
    #[error("@pad and @pad_to cannot be combined")]
    PadWithPadTo,
    #[error("@len can only be applied to struct ref, union, or unsized array fields")]
    LenOnUnsupportedType,
    #[error("@len on a fixed-size @hook field must equal the field's length")]
    LenWithFixedHook,
    #[error("@len cannot be applied to a counted or expression-sized array")]
    LenOnSizedArray,
    #[error("@len cannot be applied to a bitfield-element array")]
    LenOnBitfieldArray,
    #[error("@discriminator can only be applied to primitive and bitfield fields")]
    DiscriminatorOnNonNumeric,
    #[error("@payload can only be applied to byte-array or struct ref fields")]
    PayloadOnNonByteArray,
    #[error("a struct can declare at most one @payload field")]
    MultiplePayloads,
    #[error("@{0} cannot be applied to a @skip field")]
    HandoffOnSkip(&'static str),
    #[error("@{0} cannot be applied inside a conditional")]
    HandoffInConditional(&'static str),
    #[error("@len is not supported on concat fields")]
    LenOnConcat,
    #[error("@{0} combined with @hook is not supported")]
    HandoffWithHook(&'static str),
    #[error("@discriminator is not supported on concat and union fields")]
    DiscriminatorOnConcatOrUnion,
    #[error("@payload is not supported on concat and union fields")]
    PayloadOnConcatOrUnion,
    #[error("@cache arguments must be 'len' or 'value'")]
    InvalidCacheArg,
}

/// Padding and alignment semantics: `@pad(N)` consumes N bytes before the
/// field. `@pad_to(N)` consumes bytes until the field starts at a multiple of
/// N bytes, rounding any partial bit offset up to the next byte. `@align(N)`
/// consumes nothing; it requires the field to start at a multiple of N bytes,
/// failing codegen for fixed offsets and parsing for dynamic offsets. `@skip`
/// parses and validates the field as usual but omits it from the public
/// accessor surface.
///
/// Length bounding semantics: `@len(expr)` declares that a struct ref, union,
/// or unsized (`@greedy`/`@until`) array field occupies exactly `expr` bytes.
/// The inner content receives only that slice, so it cannot read outside its
/// bound, and the enclosing struct advances by exactly `expr` bytes regardless
/// of how many the inner content consumed. Bytes left unconsumed inside the
/// bound are not an error; they are exposed via the generated `{field}_rest()`
/// getter for higher-level dispatch. Inner content needing more than `expr`
/// bytes surfaces as `NotEnoughData` relative to the bounded slice from the
/// field getter. For `@greedy` arrays the bound is the window the array
/// consumes, so there is no rest; for `@until` arrays the sentinel must fall
/// within the window and the bytes after it are the rest. Counted or
/// expression-sized arrays and bitfield-element arrays are rejected, as they
/// already carry an intrinsic length.
///
/// Handoff semantics: `@discriminator` marks a numeric (primitive or bitfield)
/// field as a protocol dispatch key (e.g. EtherType, IP protocol number, UDP
/// port); multiple are allowed per struct and surface in declaration order.
/// `@payload` marks a byte-array (`[u8; expr]`, `@greedy`/`@until`) or struct
/// ref field as the payload; at most one per struct. A struct with a `@payload`
/// field generates `handoff()` returning `Some(Handoff { keys, payload,
/// payload_byte_range })`, letting a dependent crate chain parsers without
/// naming the concrete generated types. Both are rejected on `@skip` fields,
/// inside conditionals, and on union/concat/hook fields.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedAttrs<'a> {
    pub endian: Option<Endian>,
    pub bit_order: Option<BitOrder>,
    pub hook: Option<Hook>,
    pub check: Option<ast::Expr<'a>>,
    pub range: Option<(ast::Expr<'a>, ast::Expr<'a>)>,
    pub until: Option<u8>,
    pub greedy: bool,
    pub max_iter: Option<ast::Expr<'a>>,
    pub skip: bool,
    pub pad: Option<usize>,
    pub pad_to: Option<usize>,
    pub align: Option<usize>,
    pub len: Option<ast::Expr<'a>>,
    pub discriminator: bool,
    pub payload: bool,
    pub cache_len: bool,
    pub cache_value: bool,
}

impl<'a> ParsedAttrs<'a> {
    pub fn parse(attrs: &[ast::Attribute<'a>]) -> Result<Self, Error> {
        let mut result = Self::default();
        for attr in attrs {
            match attr.name.text {
                "endian" => result.endian = Some(Self::parse_endian(attr)?),
                "bit_order" => result.bit_order = Some(Self::parse_bit_order(attr)?),
                "hook" => result.hook = Some(Self::parse_hook(attr)?),
                "check" => result.check = Some(Self::parse_check(attr, "check")?),
                "validate" => result.check = Some(Self::parse_check(attr, "validate")?),
                "range" => result.range = Some(Self::parse_range(attr)?),
                "until" => result.until = Some(Self::parse_until(attr)?),
                "greedy" => {
                    Self::parse_greedy(attr)?;
                    result.greedy = true;
                }
                "max_iter" => result.max_iter = Some(Self::parse_check(attr, "max_iter")?),
                "skip" => {
                    if !attr.args.is_empty() {
                        return Err(Error::WrongArgCount {
                            attr: "skip",
                            expected: 0,
                            got: attr.args.len(),
                        });
                    }
                    result.skip = true;
                }
                "pad" => result.pad = Some(Self::parse_padding(attr, "pad")?),
                "len" => result.len = Some(Self::parse_check(attr, "len")?),
                "pad_to" => result.pad_to = Some(Self::parse_padding(attr, "pad_to")?),
                "align" => result.align = Some(Self::parse_padding(attr, "align")?),
                "discriminator" => {
                    Self::parse_flag(attr, "discriminator")?;
                    result.discriminator = true;
                }
                "payload" => {
                    Self::parse_flag(attr, "payload")?;
                    result.payload = true;
                }
                "cache" => Self::parse_cache(attr, &mut result)?,
                _ => {}
            }
        }
        if result.pad.is_some() && result.pad_to.is_some() {
            return Err(Error::PadWithPadTo);
        }
        Ok(result)
    }

    fn parse_flag(attr: &ast::Attribute<'_>, name: &'static str) -> Result<(), Error> {
        if attr.args.is_empty() {
            Ok(())
        } else {
            Err(Error::WrongArgCount {
                attr: name,
                expected: 0,
                got: attr.args.len(),
            })
        }
    }

    fn parse_cache(attr: &ast::Attribute<'_>, result: &mut Self) -> Result<(), Error> {
        if attr.args.is_empty() {
            result.cache_len = true;
            result.cache_value = true;
            return Ok(());
        }
        for arg in &attr.args {
            match &arg.kind {
                ast::ExprKind::Path(path) if path.as_slice() == ["len"] => result.cache_len = true,
                ast::ExprKind::Path(path) if path.as_slice() == ["value"] => result.cache_value = true,
                _ => return Err(Error::InvalidCacheArg),
            }
        }
        Ok(())
    }

    fn parse_padding(attr: &ast::Attribute<'_>, name: &'static str) -> Result<usize, Error> {
        let [arg] = attr.args.as_slice() else {
            return Err(Error::WrongArgCount {
                attr: name,
                expected: 1,
                got: attr.args.len(),
            });
        };
        match &arg.kind {
            ast::ExprKind::Literal(ast::Literal::Int(lit)) if lit.value > 0 => Ok(lit.value),
            _ => Err(Error::InvalidPaddingArg(name)),
        }
    }

    fn parse_check(attr: &ast::Attribute<'a>, name: &'static str) -> Result<ast::Expr<'a>, Error> {
        let [expr] = attr.args.as_slice() else {
            return Err(Error::WrongArgCount {
                attr: name,
                expected: 1,
                got: attr.args.len(),
            });
        };
        Ok(expr.clone())
    }

    fn parse_range(attr: &ast::Attribute<'a>) -> Result<(ast::Expr<'a>, ast::Expr<'a>), Error> {
        let [min, max] = attr.args.as_slice() else {
            return Err(Error::WrongArgCount {
                attr: "range",
                expected: 2,
                got: attr.args.len(),
            });
        };
        Ok((min.clone(), max.clone()))
    }

    fn parse_until(attr: &ast::Attribute<'_>) -> Result<u8, Error> {
        let [arg] = attr.args.as_slice() else {
            return Err(Error::WrongArgCount {
                attr: "until",
                expected: 1,
                got: attr.args.len(),
            });
        };
        match &arg.kind {
            ast::ExprKind::Literal(ast::Literal::Int(lit)) if lit.value <= usize::from(u8::MAX) => {
                Ok(lit.value as u8)
            }
            _ => Err(Error::InvalidUntilSentinel),
        }
    }

    fn parse_greedy(attr: &ast::Attribute<'_>) -> Result<(), Error> {
        if attr.args.len() != 1 {
            return Err(Error::WrongArgCount {
                attr: "greedy",
                expected: 1,
                got: attr.args.len(),
            });
        }
        match &attr.args[0].kind {
            ast::ExprKind::Path(path) if path.as_slice() == ["unsafe_eof"] => Ok(()),
            ast::ExprKind::Path(path) if path.len() == 1 => {
                Err(Error::InvalidGreedyValue(path[0].text.to_string()))
            }
            _ => Err(Error::InvalidGreedyValue("<non-identifier>".to_string())),
        }
    }

    fn parse_endian(attr: &ast::Attribute<'_>) -> Result<Endian, Error> {
        if attr.args.len() != 1 {
            return Err(Error::WrongArgCount {
                attr: "endian",
                expected: 1,
                got: attr.args.len(),
            });
        }
        match &attr.args[0].kind {
            ast::ExprKind::Path(path) if path.len() == 1 => match path[0].text {
                "big" => Ok(Endian::Big),
                "little" => Ok(Endian::Little),
                other => Err(Error::InvalidEndianValue(other.to_string())),
            },
            _ => Err(Error::InvalidEndianValue("<non-identifier>".to_string())),
        }
    }

    fn parse_bit_order(attr: &ast::Attribute<'_>) -> Result<BitOrder, Error> {
        if attr.args.len() != 1 {
            return Err(Error::WrongArgCount {
                attr: "bit_order",
                expected: 1,
                got: attr.args.len(),
            });
        }
        match &attr.args[0].kind {
            ast::ExprKind::Path(path) if path.len() == 1 => match path[0].text {
                "msb" => Ok(BitOrder::Msb),
                "lsb" => Ok(BitOrder::Lsb),
                other => Err(Error::InvalidBitOrderValue(other.to_string())),
            },
            _ => Err(Error::InvalidBitOrderValue("<non-identifier>".to_string())),
        }
    }

    fn parse_hook(attr: &ast::Attribute<'_>) -> Result<Hook, Error> {
        if attr.args.len() != 2 {
            return Err(Error::WrongArgCount {
                attr: "hook",
                expected: 2,
                got: attr.args.len(),
            });
        }
        let fn_path = Self::path_to_tokens(&attr.args[0])?;
        let return_ty = Self::path_to_tokens(&attr.args[1])?;
        Ok(Hook { fn_path, return_ty })
    }

    fn path_to_tokens(expr: &ast::Expr<'_>) -> Result<TokenStream, Error> {
        match &expr.kind {
            ast::ExprKind::Path(segments) => {
                let idents: Vec<_> = segments
                    .iter()
                    .map(|s| format_ident!("{}", s.text))
                    .collect();
                Ok(quote::quote! { #(#idents)::* })
            }
            // Raw type token (`@hook` return type): may carry references, generics,
            // slices, and lifetimes. The DSL spells path separators as `.`; rewrite
            // to `::` (a Rust type never legitimately contains a `.`) before lexing.
            ast::ExprKind::RawType(raw) => raw
                .replace('.', "::")
                .parse::<TokenStream>()
                .map_err(|_| Error::InvalidHookArg),
            _ => Err(Error::InvalidHookArg),
        }
    }

    pub fn merge_inherited(&self, default: Inherited) -> Inherited {
        Inherited {
            endian: self.endian.unwrap_or(default.endian),
            bit_order: self.bit_order.unwrap_or(default.bit_order),
        }
    }
}
