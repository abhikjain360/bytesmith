use binparse_dsl as ast;
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
    #[error("@endian cannot be applied to struct ref (struct uses its own definition's endianness)")]
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
}

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
}

impl<'a> ParsedAttrs<'a> {
    pub fn parse(attrs: &[ast::Attribute<'a>]) -> Result<Self, Error> {
        let mut result = Self::default();
        for attr in attrs {
            match attr.name {
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
                _ => {}
            }
        }
        Ok(result)
    }

    fn parse_check(
        attr: &ast::Attribute<'a>,
        name: &'static str,
    ) -> Result<ast::Expr<'a>, Error> {
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
        match arg {
            ast::Expr::Literal(ast::Literal::Int(lit)) if lit.value <= usize::from(u8::MAX) => {
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
        match &attr.args[0] {
            ast::Expr::Path(path) if path.as_slice() == ["unsafe_eof"] => Ok(()),
            ast::Expr::Path(path) if path.len() == 1 => {
                Err(Error::InvalidGreedyValue(path[0].to_string()))
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
        match &attr.args[0] {
            ast::Expr::Path(path) if path.len() == 1 => match path[0] {
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
        match &attr.args[0] {
            ast::Expr::Path(path) if path.len() == 1 => match path[0] {
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
        match expr {
            ast::Expr::Path(segments) => {
                let idents: Vec<_> = segments.iter().map(|s| format_ident!("{}", s)).collect();
                Ok(quote::quote! { #(#idents)::* })
            }
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
