//! Implementation of `#[derive(ToTeaLeaf)]`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields};

use crate::attrs::{ContainerAttrs, FieldAttrs};
use crate::schema;
use crate::util;

pub fn derive(input: &DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let container_attrs = ContainerAttrs::from_attrs(&input.attrs)?;
    let schema_name = container_attrs
        .rename
        .clone()
        .unwrap_or_else(|| name.to_string());

    let to_value_body = generate_to_value(input)?;
    let collect_schemas_body = schema::generate_collect_schemas(input)?;

    Ok(quote! {
        impl #impl_generics ::tealeaf::convert::ToTeaLeaf for #name #type_generics #where_clause {
            fn to_tealeaf_value(&self) -> ::tealeaf::Value {
                #to_value_body
            }

            #collect_schemas_body

            fn tealeaf_field_type() -> ::tealeaf::FieldType {
                ::tealeaf::FieldType::new(#schema_name)
            }
        }
    })
}

fn generate_to_value(input: &DeriveInput) -> syn::Result<TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let mut field_inserts = Vec::new();

            for field in &fields.named {
                let field_attrs = FieldAttrs::from_attrs(&field.attrs)?;
                if field_attrs.skip {
                    continue;
                }

                let field_ident = field.ident.as_ref().unwrap();
                let field_name = field_attrs
                    .rename
                    .unwrap_or_else(|| field_ident.to_string());

                let ty = &field.ty;

                if field_attrs.flatten {
                    // Flatten: merge nested object fields into parent
                    field_inserts.push(quote! {
                        if let ::tealeaf::Value::Object(inner_obj) =
                            ::tealeaf::convert::ToTeaLeaf::to_tealeaf_value(&self.#field_ident)
                        {
                            for (k, v) in inner_obj {
                                obj.insert(k, v);
                            }
                        }
                    });
                } else if let Some(ref type_str) = field_attrs.type_override {
                    // Type override: special handling
                    match type_str.as_str() {
                        "timestamp" => {
                            if util::is_option_type(ty) {
                                field_inserts.push(quote! {
                                    match &self.#field_ident {
                                        Some(v) => {
                                            obj.insert(
                                                #field_name.to_string(),
                                                ::tealeaf::Value::Timestamp(*v as i64),
                                            );
                                        }
                                        None => {
                                            obj.insert(
                                                #field_name.to_string(),
                                                ::tealeaf::Value::Null,
                                            );
                                        }
                                    }
                                });
                            } else {
                                field_inserts.push(quote! {
                                    obj.insert(
                                        #field_name.to_string(),
                                        ::tealeaf::Value::Timestamp(self.#field_ident as i64),
                                    );
                                });
                            }
                        }
                        _ => {
                            // Generic override: just use the standard conversion
                            field_inserts.push(quote! {
                                obj.insert(
                                    #field_name.to_string(),
                                    ::tealeaf::convert::ToTeaLeaf::to_tealeaf_value(&self.#field_ident),
                                );
                            });
                        }
                    }
                } else {
                    // Standard conversion
                    field_inserts.push(quote! {
                        obj.insert(
                            #field_name.to_string(),
                            ::tealeaf::convert::ToTeaLeaf::to_tealeaf_value(&self.#field_ident),
                        );
                    });
                }
            }

            Ok(quote! {
                let mut obj = ::std::collections::HashMap::new();
                #(#field_inserts)*
                ::tealeaf::Value::Object(obj)
            })
        }
        Data::Enum(data_enum) => generate_enum_to_value(input, data_enum),
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "ToTeaLeaf can only be derived for structs with named fields or enums",
        )),
    }
}

fn generate_enum_to_value(input: &DeriveInput, data: &DataEnum) -> syn::Result<TokenStream> {
    let name = &input.ident;
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

                match_arms.push(quote! {
                    #name::#variant_ident { #(ref #field_idents),* } => {
                        let mut obj = ::std::collections::HashMap::new();
                        #(
                            obj.insert(
                                #field_names.to_string(),
                                ::tealeaf::convert::ToTeaLeaf::to_tealeaf_value(#field_idents),
                            );
                        )*
                        ::tealeaf::Value::Tagged(
                            #variant_name.to_string(),
                            Box::new(::tealeaf::Value::Object(obj)),
                        )
                    }
                });
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() == 1 {
                    match_arms.push(quote! {
                        #name::#variant_ident(ref inner) => {
                            ::tealeaf::Value::Tagged(
                                #variant_name.to_string(),
                                Box::new(::tealeaf::convert::ToTeaLeaf::to_tealeaf_value(inner)),
                            )
                        }
                    });
                } else {
                    let field_bindings: Vec<syn::Ident> = (0..fields.unnamed.len())
                        .map(|i| syn::Ident::new(&format!("f{}", i), proc_macro2::Span::call_site()))
                        .collect();
                    match_arms.push(quote! {
                        #name::#variant_ident(#(ref #field_bindings),*) => {
                            ::tealeaf::Value::Tagged(
                                #variant_name.to_string(),
                                Box::new(::tealeaf::Value::Array(vec![
                                    #(::tealeaf::convert::ToTeaLeaf::to_tealeaf_value(#field_bindings)),*
                                ])),
                            )
                        }
                    });
                }
            }
            Fields::Unit => {
                match_arms.push(quote! {
                    #name::#variant_ident => {
                        ::tealeaf::Value::Tagged(
                            #variant_name.to_string(),
                            Box::new(::tealeaf::Value::Null),
                        )
                    }
                });
            }
        }
    }

    Ok(quote! {
        match self {
            #(#match_arms)*
        }
    })
}
