use std::collections::HashMap;

use binparse_dsl as ast;
use proc_macro2::TokenStream;

use crate::Len;

pub(crate) struct StructCtx<'a> {
    pub(crate) origin: &'a ast::Struct<'a>,
    pub(crate) offset: Option<Len>,
    pub(crate) done: &'a HashMap<&'a str, GeneratedStruct>,
}

pub(crate) struct GeneratedStruct {
    pub(crate) len: Option<Len>,
    pub(crate) tokens: TokenStream,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {}

impl<'a> StructCtx<'a> {
    pub(crate) fn new(
        origin: &'a ast::Struct<'a>,
        done: &'a HashMap<&'a str, GeneratedStruct>,
    ) -> Self {
        Self {
            origin,
            offset: Some(Len { byte: 0, bit: 0 }),
            done,
        }
    }

    pub(crate) fn generate(self) -> Result<GeneratedStruct, Error> {
        todo!()
    }
}
