use binparse_dsl as ast;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashMap;

use crate::{
    GeneratedLen,
    attr::{Inherited, ParsedAttrs},
    expr,
    field::{FieldAccum, getter_visibility},
    struct_::{self, DoneFieldType, GeneratedStruct, StructAccum},
    type_::{self, GeneratedTree, GeneratedTypeInfo, LenBound},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("union has no arguments")]
    NoArguments,
    #[error("union has no variants")]
    NoVariants,
    #[error("matcher has {got} elements but union has {expected} arguments")]
    MatcherCountMismatch { expected: usize, got: usize },
    #[error("union is not exhaustive: add a wildcard variant or a wildcard @error variant")]
    NonExhaustive,
    #[error("@error variant '{0}' is not declared in an error block")]
    UnknownErrorVariant(String),
    #[error("@error variant '{variant}' is missing field '{field}'")]
    MissingErrorField { variant: String, field: String },
    #[error("@error variant '{variant}' has no declared field '{field}'")]
    UnknownErrorField { variant: String, field: String },
    #[error("union variant '{name}': {error}")]
    VariantStruct {
        name: String,
        #[source]
        error: Box<crate::struct_::Error>,
    },
    #[error("non-integer literal matcher is not supported")]
    NonIntegerMatcher,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn generate<'a>(
    union: &ast::Union<'a>,
    done: &HashMap<&'a str, GeneratedStruct>,
    struct_accum: &mut StructAccum<'a>,
    accum: &mut FieldAccum,
    start_offset: GeneratedLen,
    inherited: Inherited,
    attrs: &ParsedAttrs<'a>,
    errors: &[ast::ErrorVariant<'_>],
) -> Result<GeneratedTypeInfo, type_::Error> {
    let num_args = union.args.len();
    if num_args == 0 {
        return Err(type_::Error::Union(Error::NoArguments));
    }
    if union.variants.is_empty() {
        return Err(type_::Error::Union(Error::NoVariants));
    }
    if !union
        .variants
        .iter()
        .flat_map(|variant| &variant.matchers)
        .any(is_catch_all)
    {
        return Err(type_::Error::Union(Error::NonExhaustive));
    }

    let match_expr = expr::lower_discriminants(&union.args, &struct_accum.done_fields)?;

    let start_byte: TokenStream = match &start_offset {
        GeneratedLen::Fixed(offset) => {
            if offset.bit != 0 {
                return Err(type_::Error::InvalidAlignment(*offset));
            }
            let byte = offset.byte;
            quote! { #byte }
        }
        GeneratedLen::Dynamic(tokens) => {
            quote! { (#tokens).byte }
        }
    };

    let clamped_start = quote! { (#start_byte).min(self.data.len()) };

    let bound = type_::len_bound(&start_byte, attrs, struct_accum)?;
    let (parse_slice, check_slice) = match &bound {
        Some(LenBound { end, .. }) => (
            quote! { &self.data[#start_byte..(#end)] },
            quote! { &self.data[#clamped_start..(#end)] },
        ),
        None => (
            quote! { &self.data[#start_byte..] },
            quote! { &self.data[#clamped_start..] },
        ),
    };

    let parent_struct_name = &struct_accum.name;
    let field_name = &accum.field_name;
    let field_name_str = field_name.to_string();
    let getter = accum.tree_getter.clone();
    let enum_name = format_ident!("{}_{}", parent_struct_name, field_name);

    let has_error_arms = union
        .variants
        .iter()
        .any(|variant| matches!(variant.body, ast::UnionBody::Error(..)));

    let mut variant_structs = TokenStream::new();
    let mut enum_variants = TokenStream::new();
    let mut match_arms = TokenStream::new();
    let mut len_match_arms = TokenStream::new();
    let mut check_match_arms = TokenStream::new();
    let mut rest_match_arms = TokenStream::new();
    let mut tree_arms = TokenStream::new();

    for variant in &union.variants {
        let matchers = generate_matchers(&variant.matchers, num_args)?;

        match &variant.body {
            ast::UnionBody::NamedInline(inline_struct) => {
                let variant_ident = format_ident!("{}", inline_struct.name);
                let struct_name = format_ident!(
                    "{}_{}_{}",
                    parent_struct_name,
                    field_name,
                    inline_struct.name
                );

                let variant_attrs = crate::attr::ParsedAttrs::parse(&inline_struct.attributes)?;
                let variant_inherited = variant_attrs.merge_inherited(inherited);

                let generated = struct_::generate_struct(
                    &struct_name.to_string(),
                    &inline_struct.items,
                    variant_inherited,
                    variant_attrs.len.clone(),
                    done,
                    errors,
                    quote! { #[allow(non_camel_case_types)] },
                )
                .map_err(|error| {
                    type_::Error::Union(Error::VariantStruct {
                        name: inline_struct.name.to_string(),
                        error: Box::new(error),
                    })
                })?;
                variant_structs.extend(generated.tokens);

                enum_variants.extend(quote! {
                    #variant_ident(#struct_name<'a>),
                });

                let parse_variant = quote! {
                    #struct_name::parse(#parse_slice)
                        .map(|(value, _)| #enum_name::#variant_ident(value))
                };
                rest_match_arms.extend(quote! {
                    #matchers => #struct_name::parse(#parse_slice).map(|(_, rest)| rest),
                });
                if has_error_arms {
                    match_arms.extend(quote! {
                        #matchers => #parse_variant.map_err(Error::Parse),
                    });
                } else {
                    match_arms.extend(quote! {
                        #matchers => #parse_variant,
                    });
                }

                let variant_len = match generated.len {
                    GeneratedLen::Fixed(len) => {
                        let byte = len.byte;
                        let bit = len.bit;
                        quote! { ::binparse::Len { byte: #byte, bit: #bit } }
                    }
                    GeneratedLen::Dynamic(_) => {
                        let last_offset_getter = generated
                            .last_offset_getter
                            .expect("dynamic-length variant struct has an offset getter");
                        let variant_inits = generated.cache_inits.clone();
                        let ctor = if variant_inits.is_empty() {
                            quote! { #struct_name { data: &self.data[#clamped_start..] } }
                        } else {
                            quote! {
                                #struct_name { data: &self.data[#clamped_start..], #variant_inits }
                            }
                        };
                        quote! { #ctor.#last_offset_getter() }
                    }
                };
                len_match_arms.extend(quote! {
                    #matchers => #variant_len,
                });

                check_match_arms.extend(quote! {
                    #matchers => {
                        #struct_name::parse(#check_slice)?;
                    }
                });

                let variant_name = inline_struct.name.to_string();
                tree_arms.extend(quote! {
                    Ok(#enum_name::#variant_ident(value)) => {
                        let inner = value.field_tree().renamed(#variant_name).shifted(bit_range.start);
                        ::binparse::FieldNode::new(
                                #field_name_str,
                                "union",
                                bit_range.clone(),
                                ::binparse::Value::UnionVariant(#variant_name),
                            )
                            .with_children(::std::vec![inner])
                    }
                });
            }

            ast::UnionBody::Error(error_name, fields) => {
                let declared = errors
                    .iter()
                    .find(|declared| declared.name == *error_name)
                    .ok_or_else(|| {
                        type_::Error::Union(Error::UnknownErrorVariant(error_name.to_string()))
                    })?;

                for (provided, _) in fields {
                    if !declared.fields.iter().any(|(name, _)| name == provided) {
                        return Err(type_::Error::Union(Error::UnknownErrorField {
                            variant: error_name.to_string(),
                            field: provided.to_string(),
                        }));
                    }
                }

                let mut field_inits = TokenStream::new();
                for (declared_name, primitive) in &declared.fields {
                    let value = fields
                        .iter()
                        .find(|(provided, _)| provided == declared_name)
                        .map(|(_, value)| value)
                        .ok_or_else(|| {
                            type_::Error::Union(Error::MissingErrorField {
                                variant: error_name.to_string(),
                                field: declared_name.to_string(),
                            })
                        })?;
                    let lowered =
                        expr::lower(value, expr::ExprType::Numeric, &struct_accum.done_fields)?
                            .tokens;
                    let declared_ident = format_ident!("{}", declared_name);
                    let ty = crate::match_primitive(primitive).1;
                    field_inits.extend(quote! {
                        #declared_ident: (#lowered) as #ty,
                    });
                }

                let error_ident = format_ident!("{}", error_name);
                let error_value = if declared.fields.is_empty() {
                    quote! { Error::#error_ident }
                } else {
                    quote! { Error::#error_ident { #field_inits } }
                };
                match_arms.extend(quote! {
                    #matchers => Err(#error_value),
                });
                len_match_arms.extend(quote! {
                    #matchers => ::binparse::Len::ZERO,
                });
                check_match_arms.extend(quote! {
                    #matchers => {}
                });
                rest_match_arms.extend(quote! {
                    #matchers => Ok(&self.data[#clamped_start..#clamped_start]),
                });
            }
        }
    }

    struct_accum.other_entities.extend(quote! {
        #variant_structs

        #[allow(non_camel_case_types)]
        pub enum #enum_name<'a> {
            #enum_variants
        }
    });

    let check_fn_name = format_ident!("{}_union_check", field_name);
    accum.helper_fns.extend(quote! {
        fn #check_fn_name(&self) -> Result<(), ::binparse::ParseError> {
            match #match_expr {
                #check_match_arms
            }
            Ok(())
        }
    });
    accum.pre_length_checks.extend(quote! {
        self.#check_fn_name()?;
    });

    let field_getter_body = quote! {
        match #match_expr {
            #match_arms
        }
    };

    let return_ty = if has_error_arms {
        quote! { Result<#enum_name<'a>, Error> }
    } else {
        quote! { ::binparse::ParseResult<#enum_name<'a>> }
    };

    let rest_fn_name = format_ident!("{}_rest", field_name);
    let len = match &bound {
        Some(LenBound { field_len, .. }) => {
            let (vis, dead_code) = getter_visibility(attrs);
            accum.helper_fns.extend(quote! {
                #dead_code
                #vis fn #rest_fn_name(&self) -> ::binparse::ParseResult<&'a [u8]> {
                    match #match_expr {
                        #rest_match_arms
                    }
                }
            });
            field_len.clone()
        }
        None => GeneratedLen::Dynamic(quote! {
            match #match_expr {
                #len_match_arms
            }
        }),
    };

    let err_arm = if has_error_arms {
        quote! {
            Err(Error::Parse(error)) => ::binparse::FieldNode::new(
                    #field_name_str,
                    "union",
                    bit_range.clone(),
                    ::binparse::Value::Opaque,
                )
                .with_status(::binparse::Status::Error(error)),
            Err(error) => ::binparse::FieldNode::new(
                    #field_name_str,
                    "union",
                    bit_range.clone(),
                    ::binparse::Value::Opaque,
                )
                .with_status(::binparse::Status::Failed(error.variant_name())),
        }
    } else {
        quote! {
            Err(error) => ::binparse::FieldNode::new(
                    #field_name_str,
                    "union",
                    bit_range.clone(),
                    ::binparse::Value::Opaque,
                )
                .with_status(::binparse::Status::Error(error)),
        }
    };

    let tree = GeneratedTree::Node(match &bound {
        Some(_) => quote! {
            {
                let mut node = match self.#getter() {
                    #tree_arms
                    #err_arm
                };
                if let Ok(rest) = self.#rest_fn_name()
                    && !rest.is_empty()
                {
                    let consumed = node
                        .children
                        .last()
                        .map(|child| child.bit_range.end)
                        .unwrap_or(bit_range.start)
                        .min(bit_range.end);
                    node.children.push(::binparse::FieldNode::new(
                        "rest",
                        "[u8]",
                        consumed..bit_range.end,
                        ::binparse::Value::Bytes(rest),
                    ));
                }
                node
            }
        },
        None => quote! {
            match self.#getter() {
                #tree_arms
                #err_arm
            }
        },
    });

    Ok(GeneratedTypeInfo {
        len,
        field_getter_body,
        return_ty,
        field_type: DoneFieldType::Other,
        tree,
    })
}

fn is_catch_all(matcher: &ast::UnionMatcher<'_>) -> bool {
    match matcher {
        ast::UnionMatcher::Wildcard => true,
        ast::UnionMatcher::Tuple(elements) => elements.iter().all(is_catch_all),
        ast::UnionMatcher::Literal(_) => false,
    }
}

fn generate_matchers(
    matchers: &[ast::UnionMatcher<'_>],
    num_args: usize,
) -> Result<TokenStream, Error> {
    let patterns = matchers
        .iter()
        .map(|matcher| {
            validate_matcher_arity(matcher, num_args)?;
            generate_matcher(matcher)
        })
        .collect::<Result<Vec<_>, Error>>()?;

    Ok(quote! { #(#patterns)|* })
}

fn validate_matcher_arity(matcher: &ast::UnionMatcher<'_>, num_args: usize) -> Result<(), Error> {
    let got = match matcher {
        ast::UnionMatcher::Wildcard => return Ok(()),
        ast::UnionMatcher::Literal(_) => 1,
        ast::UnionMatcher::Tuple(elements) => elements.len(),
    };
    if got != num_args {
        return Err(Error::MatcherCountMismatch {
            expected: num_args,
            got,
        });
    }
    Ok(())
}

fn generate_matcher(matcher: &ast::UnionMatcher<'_>) -> Result<TokenStream, Error> {
    match matcher {
        ast::UnionMatcher::Literal(ast::Literal::Int(int_lit)) => {
            let value = proc_macro2::Literal::usize_unsuffixed(int_lit.value);
            Ok(quote! { #value })
        }
        ast::UnionMatcher::Literal(_) => Err(Error::NonIntegerMatcher),
        ast::UnionMatcher::Wildcard => Ok(quote! { _ }),
        ast::UnionMatcher::Tuple(elements) => {
            let element_patterns = elements
                .iter()
                .map(generate_matcher)
                .collect::<Result<Vec<_>, Error>>()?;
            Ok(quote! { (#(#element_patterns),*) })
        }
    }
}
