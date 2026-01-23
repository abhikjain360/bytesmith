use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::format_ident;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Endian {
    #[default]
    Big,
    Little,
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
    #[error("@endian cannot be applied to u8 (single byte has no endianness)")]
    EndianOnU8,
    #[error("@endian cannot be applied to bitfields")]
    EndianOnBitfield,
    #[error("@endian cannot be applied to struct ref (struct uses its own definition's endianness)")]
    EndianOnStructRef,
    #[error("@hook arguments must be paths (fn_name, ReturnType)")]
    InvalidHookArg,
    #[error("@hook on VLA requires [u8] type")]
    HookVlaNotU8,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedAttrs {
    pub endian: Option<Endian>,
    pub hook: Option<Hook>,
}

impl ParsedAttrs {
    pub fn parse(attrs: &[ast::Attribute<'_>]) -> Result<Self, Error> {
        let mut result = Self::default();
        for attr in attrs {
            match attr.name {
                "endian" => result.endian = Some(Self::parse_endian(attr)?),
                "hook" => result.hook = Some(Self::parse_hook(attr)?),
                _ => {}
            }
        }
        Ok(result)
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

    pub fn merge_endian(&self, default: Endian) -> Endian {
        self.endian.unwrap_or(default)
    }
}
