use binparse_dsl as ast;
use quote::quote;

use crate::{
    GeneratedLen,
    attr::Endian,
    struct_::DoneFieldType,
    type_::{Error, GeneratedTypeInfo},
};

pub(crate) fn generate(
    primitive: ast::Primitive,
    start_offset: GeneratedLen,
    endian: Endian,
) -> Result<GeneratedTypeInfo, Error> {
    let (len, def) = crate::match_primitive(&primitive);
    let return_ty = def.clone();
    let byte_len = len.byte;

    let from_bytes = match endian {
        Endian::Big => quote! { from_be_bytes },
        Endian::Little => quote! { from_le_bytes },
    };

    let field_getter_body = match start_offset {
        GeneratedLen::Fixed(offset) => {
            if offset.bit != 0 {
                return Err(Error::InvalidAlignment(offset));
            }

            let start_byte = offset.byte;

            if matches!(primitive, ast::Primitive::U8) {
                quote! { self.data[#start_byte] }
            } else {
                let end_byte = offset.byte + len.byte;
                quote! {
                    #def::#from_bytes(self.data[#start_byte..#end_byte].try_into().unwrap())
                }
            }
        }

        GeneratedLen::Dynamic(offset_expr) => {
            if matches!(primitive, ast::Primitive::U8) {
                quote! {
                    {
                        let offset = #offset_expr;
                        debug_assert!(offset.bit == 0, "primitive requires byte alignment");
                        self.data[offset.byte]
                    }
                }
            } else {
                quote! {
                    {
                        let offset = #offset_expr;
                        debug_assert!(offset.bit == 0, "primitive requires byte alignment");
                        let start = offset.byte;
                        let end = start + #byte_len;
                        #def::#from_bytes(self.data[start..end].try_into().unwrap())
                    }
                }
            }
        }
    };

    Ok(GeneratedTypeInfo {
        len: GeneratedLen::Fixed(len),
        field_getter_body,
        return_ty,
        field_type: DoneFieldType::Primitive,
    })
}
