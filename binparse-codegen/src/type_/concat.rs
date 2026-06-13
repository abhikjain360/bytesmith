use std::collections::HashMap;

use binparse::Len;
use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::{
    GeneratedLen,
    attr::{Inherited, ParsedAttrs},
    field::FieldAccum,
    struct_::{DoneFieldType, GeneratedStruct, StructAccum},
    type_::{self, GeneratedTree, GeneratedTypeInfo},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("concat item {0} must have known length")]
    UnknownItemLen(usize),
    #[error(transparent)]
    Attr(#[from] crate::attr::Error),
}

pub(crate) fn generate<'a>(
    items: &[ast::ConcatItem<'a>],
    done: &HashMap<&'a str, GeneratedStruct>,
    struct_accum: &mut StructAccum<'a>,
    field_accum: &mut FieldAccum,
    start_offset: GeneratedLen,
    inherited: Inherited,
    errors: &[ast::ErrorVariant<'_>],
) -> Result<GeneratedTypeInfo, type_::Error> {
    let mut total_len = GeneratedLen::Fixed(Len::default());
    let mut field_types = Vec::new();
    let mut field_exprs = TokenStream::new();
    let mut tree_items = TokenStream::new();

    let mut current_offset = start_offset;
    let parent_field_name = field_accum.field_name.clone();
    let parent_tree_getter = field_accum.tree_getter.clone();

    for (i, item) in items.iter().enumerate() {
        let item_name = format_ident!("{}_{}", parent_field_name, i);

        let item_attrs = ParsedAttrs::parse(&item.attributes)?;
        let item_inherited = item_attrs.merge_inherited(inherited);

        field_accum.field_name = item_name.clone();
        field_accum.tree_getter = item_name.clone();
        let info = type_::generate(
            &item.ty,
            done,
            struct_accum,
            field_accum,
            current_offset.clone(),
            item_inherited,
            &item_attrs,
            errors,
        );
        field_accum.field_name = parent_field_name.clone();
        field_accum.tree_getter = parent_tree_getter.clone();
        let info = info?;

        let item_node = info.tree_node(&item_name, &item_name);
        let return_ty = info.return_ty;
        let field_getter_body = info.field_getter_body;
        if item_attrs.skip {
            field_accum.helper_fns.extend(quote! {
                #[allow(dead_code)]
                #[allow(clippy::identity_op)]
                fn #item_name(&self) -> #return_ty {
                    #field_getter_body
                }
            });
        } else {
            field_accum.helper_fns.extend(quote! {
                #[allow(clippy::identity_op)]
                pub fn #item_name(&self) -> #return_ty {
                    #field_getter_body
                }
            });

            field_types.push(return_ty);
            field_exprs.extend(quote! { self.#item_name(), });
        }

        let hide = item_attrs.skip.then(|| quote! { .hide() });
        let item_len = info.len;
        let item_end = current_offset.clone() + item_len.clone();
        let item_start_bits = len_bits(&current_offset);
        let item_end_bits = len_bits(&item_end);
        tree_items.extend(quote! {
            {
                let bit_range = #item_start_bits..#item_end_bits;
                item_nodes.push(#item_node #hide);
            }
        });

        total_len = total_len + item_len.clone();
        current_offset = item_len + current_offset;
    }

    let field_getter_body = quote! {
        ( #field_exprs )
    };

    let return_ty = quote! { ( #(#field_types,)* ) };

    let parent_name_str = parent_field_name.to_string();
    let tree = GeneratedTree::Node(quote! {
        {
            let mut item_nodes = ::std::vec::Vec::new();
            #tree_items
            ::binparse::FieldNode::new(#parent_name_str, "concat", bit_range.clone(), ::binparse::Value::Struct)
                .with_children(item_nodes)
        }
    });

    Ok(GeneratedTypeInfo {
        len: total_len,
        field_getter_body,
        return_ty,
        field_type: DoneFieldType::Other,
        tree,
    })
}

fn len_bits(len: &GeneratedLen) -> TokenStream {
    match len {
        GeneratedLen::Fixed(len) => {
            let bits = len.bits();
            quote! { #bits }
        }
        GeneratedLen::Dynamic(tokens) => quote! { ({ #tokens }).bits() },
    }
}
