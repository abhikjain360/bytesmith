use binparse_dsl as ast;
use quote::quote;

use crate::{
    GeneratedLen,
    attr::Endian,
    struct_::DoneFieldType,
    type_::{Error, GeneratedTree, GeneratedTypeInfo},
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

    let single_byte = crate::single_byte_read(&primitive);

    let field_getter_body = match start_offset {
        GeneratedLen::Fixed(offset) => {
            if offset.bit != 0 {
                return Err(Error::InvalidAlignment(offset));
            }

            let start_byte = offset.byte;

            if let Some(read) = &single_byte {
                quote! { self.data[#start_byte] #read }
            } else {
                let end_byte = offset.byte + len.byte;
                quote! {
                    #def::#from_bytes(self.data[#start_byte..#end_byte].try_into().unwrap())
                }
            }
        }

        GeneratedLen::Dynamic(offset_expr) => {
            if let Some(read) = &single_byte {
                quote! {
                    {
                        let offset = #offset_expr;
                        debug_assert!(offset.bit == 0, "primitive requires byte alignment");
                        self.data[offset.byte] #read
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

    let type_name = return_ty.to_string();
    let tree = if crate::is_signed(&primitive) {
        GeneratedTree::Int(type_name)
    } else {
        GeneratedTree::UInt(type_name)
    };

    Ok(GeneratedTypeInfo {
        len: GeneratedLen::Fixed(len),
        field_getter_body,
        return_ty,
        field_type: DoneFieldType::Primitive,
        tree,
    })
}
