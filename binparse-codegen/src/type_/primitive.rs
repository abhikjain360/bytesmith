use binparse_dsl as ast;
use quote::quote;

use crate::{
    GeneratedLen,
    type_::{Error, GeneratedType},
};

pub(crate) struct PrimitiveCtx<'a> {
    pub(crate) primitive: &'a ast::Primitive,
    pub(crate) start_offset: GeneratedLen,
}

impl PrimitiveCtx<'_> {
    pub(crate) fn generate(self) -> Result<GeneratedType, Error> {
        let (len, def) = crate::match_primitive(self.primitive);
        let return_ty = def.clone();

        match self.start_offset {
            GeneratedLen::Fixed(offset) => {
                if offset.bit != 0 {
                    return Err(Error::InvalidAlignment(offset));
                }

                let end = offset + len;
                let start_byte = offset.byte;
                let end_byte = end.byte;

                let field_getter_body = quote! {
                    #def::from_ne_bytes(self.data[#start_byte..#end_byte].try_into().unwrap())
                };

                Ok(GeneratedType {
                    len: GeneratedLen::Fixed(len),
                    definitions: quote! {},
                    helper_fns: quote! {},
                    helper_entities: quote! {},
                    field_getter_body,
                    return_ty,
                })
            }

            GeneratedLen::Dynamic(_) => todo!(),
        }
    }
}
