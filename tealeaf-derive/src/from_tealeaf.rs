//! Implementation of `#[derive(FromTeaLeaf)]`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields};

use crate::attrs::FieldAttrs;
use crate::util;

pub fn derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let from_value_body = generate_from_value(input)?;

    Ok(quote! {
        impl #impl_generics ::tealeaf::convert::FromTeaLeaf for #name #type_generics #where_clause {
            fn from_tealeaf_value(value: &::tealeaf::Value) -> ::std::result::Result<Self, ::tealeaf::convert::ConvertError> {
                #from_value_body
            }
        }
    })
}

fn generate_from_value(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let name_str = name.to_string();

    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let mut field_extractions = Vec::new();

            for field in &fields.named {
                let field_attrs = FieldAttrs::from_attrs(&field.attrs)?;
                let field_ident = field.ident.as_ref().unwrap();
                let field_ident_str = field_ident.to_string();
                let ty = &field.ty;

                if field_attrs.skip {
                    // Skipped field: must use Default
                    if field_attrs.default {
                        if let Some(ref expr_str) = field_attrs.default_expr {
                            let expr: syn::Expr = syn::parse_str(expr_str)?;
                            field_extractions.push(quote! {
                                #field_ident: #expr,
                            });
                        } else {
                            field_extractions.push(quote! {
                                #field_ident: ::std::default::Default::default(),
                            });
                        }
                    } else {
                        field_extractions.push(quote! {
                            #field_ident: ::std::default::Default::default(),
                        });
                    }
                    continue;
                }

                let field_name = field_attrs
                    .rename
                    .clone()
                    .unwrap_or_else(|| field_ident_str.clone());

                if field_attrs.flatten {
                    // Flatten: pass the entire object to the nested type
                    field_extractions.push(quote! {
                        #field_ident: <#ty as ::tealeaf::convert::FromTeaLeaf>::from_tealeaf_value(value)
                            .map_err(|e| ::tealeaf::convert::ConvertError::Nested {
                                path: format!("{}.{}", #name_str, #field_name),
                                source: Box::new(e),
                            })?,
                    });
                    continue;
                }

                let is_option = util::is_option_type(ty);

                if let Some(ref type_str) = field_attrs.type_override {
                    // Type override handling
                    match type_str.as_str() {
                        "timestamp" if is_option => {
                            field_extractions.push(quote! {
                                #field_ident: {
                                    match obj.get(#field_name) {
                                        Some(v) if !v.is_null() => {
                                            Some(v.as_timestamp().or_else(|| v.as_int())
                                                .ok_or_else(|| ::tealeaf::convert::ConvertError::TypeMismatch {
                                                    expected: "timestamp".into(),
                                                    got: format!("{:?}", v.tl_type()),
                                                    path: format!("{}.{}", #name_str, #field_name),
                                                })?)
                                        }
                                        _ => None,
                                    }
                                },
                            });
                        }
                        "timestamp" => {
                            field_extractions.push(quote! {
                                #field_ident: {
                                    let v = obj.get(#field_name).ok_or_else(|| ::tealeaf::convert::ConvertError::MissingField {
                                        struct_name: #name_str.into(),
                                        field: #field_name.into(),
                                    })?;
                                    v.as_timestamp().or_else(|| v.as_int())
                                        .ok_or_else(|| ::tealeaf::convert::ConvertError::TypeMismatch {
                                            expected: "timestamp".into(),
                                            got: format!("{:?}", v.tl_type()),
                                            path: format!("{}.{}", #name_str, #field_name),
                                        })? as _
                                },
                            });
                        }
                        _ => {
                            // Generic type override: use standard conversion
                            field_extractions
                                .push(generate_standard_field(name_str.clone(), &field_name, field_ident, ty, is_option, &field_attrs)?);
                        }
                    }
                } else {
                    field_extractions
                        .push(generate_standard_field(name_str.clone(), &field_name, field_ident, ty, is_option, &field_attrs)?);
                }
            }

            Ok(quote! {
                let obj = value.as_object().ok_or_else(|| ::tealeaf::convert::ConvertError::TypeMismatch {
                    expected: "object".into(),
                    got: format!("{:?}", value.tl_type()),
                    path: #name_str.into(),
                })?;

                Ok(Self {
                    #(#field_extractions)*
                })
            })
        }
        Data::Enum(data_enum) => generate_enum_from_value(input, data_enum),
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "FromTeaLeaf can only be derived for structs with named fields or enums",
        )),
    }
}

fn generate_standard_field(
    struct_name: String,
    field_name: &str,
    field_ident: &syn::Ident,
    ty: &syn::Type,
    is_option: bool,
    attrs: &FieldAttrs,
) -> syn::Result<TokenStream> {
    if is_option {
        Ok(quote! {
            #field_ident: {
                match obj.get(#field_name) {
                    Some(v) if !v.is_null() => {
                        Some(<_ as ::tealeaf::convert::FromTeaLeaf>::from_tealeaf_value(v)
                            .map_err(|e| ::tealeaf::convert::ConvertError::Nested {
                                path: format!("{}.{}", #struct_name, #field_name),
                                source: Box::new(e),
                            })?)
                    }
                    _ => None,
                }
            },
        })
    } else if attrs.default {
        if let Some(ref expr_str) = attrs.default_expr {
            let expr: syn::Expr = syn::parse_str(expr_str)?;
            Ok(quote! {
                #field_ident: {
                    match obj.get(#field_name) {
                        Some(v) if !v.is_null() => {
                            <#ty as ::tealeaf::convert::FromTeaLeaf>::from_tealeaf_value(v)
                                .map_err(|e| ::tealeaf::convert::ConvertError::Nested {
                                    path: format!("{}.{}", #struct_name, #field_name),
                                    source: Box::new(e),
                                })?
                        }
                        _ => #expr,
                    }
                },
            })
        } else {
            Ok(quote! {
                #field_ident: {
                    match obj.get(#field_name) {
                        Some(v) if !v.is_null() => {
                            <#ty as ::tealeaf::convert::FromTeaLeaf>::from_tealeaf_value(v)
                                .map_err(|e| ::tealeaf::convert::ConvertError::Nested {
                                    path: format!("{}.{}", #struct_name, #field_name),
                                    source: Box::new(e),
                                })?
                        }
                        _ => ::std::default::Default::default(),
                    }
                },
            })
        }
    } else {
        Ok(quote! {
            #field_ident: {
                let v = obj.get(#field_name).ok_or_else(|| ::tealeaf::convert::ConvertError::MissingField {
                    struct_name: #struct_name.into(),
                    field: #field_name.into(),
                })?;
                <#ty as ::tealeaf::convert::FromTeaLeaf>::from_tealeaf_value(v)
                    .map_err(|e| ::tealeaf::convert::ConvertError::Nested {
                        path: format!("{}.{}", #struct_name, #field_name),
                        source: Box::new(e),
                    })?
            },
        })
    }
}

fn generate_enum_from_value(input: &DeriveInput, data: &DataEnum) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let name_str = name.to_string();
    let mut match_arms = Vec::new();

    for variant in &data.variants {
        let variant_ident = &variant.ident;
        let variant_name = variant_ident.to_string();

        match &variant.fields {
            Fields::Named(fields) => {
                let field_idents: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();
                let field_names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap().to_string())
                    .collect();
                let field_types: Vec<_> = fields.named.iter().map(|f| &f.ty).collect();

                match_arms.push(quote! {
                    #variant_name => {
                        let inner_obj = inner.as_object().ok_or_else(|| ::tealeaf::convert::ConvertError::TypeMismatch {
                            expected: "object".into(),
                            got: format!("{:?}", inner.tl_type()),
                            path: format!("{}::{}", #name_str, #variant_name),
                        })?;
                        Ok(#name::#variant_ident {
                            #(
                                #field_idents: {
                                    let v = inner_obj.get(#field_names).ok_or_else(|| ::tealeaf::convert::ConvertError::MissingField {
                                        struct_name: format!("{}::{}", #name_str, #variant_name),
                                        field: #field_names.into(),
                                    })?;
                                    <#field_types as ::tealeaf::convert::FromTeaLeaf>::from_tealeaf_value(v)
                                        .map_err(|e| ::tealeaf::convert::ConvertError::Nested {
                                            path: format!("{}::{}.{}", #name_str, #variant_name, #field_names),
                                            source: Box::new(e),
                                        })?
                                },
                            )*
                        })
                    }
                });
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() == 1 {
                    let ty = &fields.unnamed[0].ty;
                    match_arms.push(quote! {
                        #variant_name => {
                            let inner_val = <#ty as ::tealeaf::convert::FromTeaLeaf>::from_tealeaf_value(inner)
                                .map_err(|e| ::tealeaf::convert::ConvertError::Nested {
                                    path: format!("{}::{}", #name_str, #variant_name),
                                    source: Box::new(e),
                                })?;
                            Ok(#name::#variant_ident(inner_val))
                        }
                    });
                } else {
                    let field_types: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();
                    let indices: Vec<usize> = (0..field_types.len()).collect();
                    match_arms.push(quote! {
                        #variant_name => {
                            let arr = inner.as_array().ok_or_else(|| ::tealeaf::convert::ConvertError::TypeMismatch {
                                expected: "array".into(),
                                got: format!("{:?}", inner.tl_type()),
                                path: format!("{}::{}", #name_str, #variant_name),
                            })?;
                            Ok(#name::#variant_ident(
                                #(
                                    <#field_types as ::tealeaf::convert::FromTeaLeaf>::from_tealeaf_value(
                                        arr.get(#indices).ok_or_else(|| ::tealeaf::convert::ConvertError::MissingField {
                                            struct_name: format!("{}::{}", #name_str, #variant_name),
                                            field: format!("index {}", #indices),
                                        })?
                                    ).map_err(|e| ::tealeaf::convert::ConvertError::Nested {
                                        path: format!("{}::{}[{}]", #name_str, #variant_name, #indices),
                                        source: Box::new(e),
                                    })?,
                                )*
                            ))
                        }
                    });
                }
            }
            Fields::Unit => {
                match_arms.push(quote! {
                    #variant_name => Ok(#name::#variant_ident),
                });
            }
        }
    }

    Ok(quote! {
        match value {
            ::tealeaf::Value::Tagged(tag, inner) => {
                match tag.as_str() {
                    #(#match_arms)*
                    other => Err(::tealeaf::convert::ConvertError::Custom(
                        format!("Unknown {} variant: {}", #name_str, other),
                    )),
                }
            }
            _ => Err(::tealeaf::convert::ConvertError::TypeMismatch {
                expected: "tagged".into(),
                got: format!("{:?}", value.tl_type()),
                path: #name_str.into(),
            }),
        }
    })
}
