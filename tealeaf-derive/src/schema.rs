//! Schema generation helpers for derive macros.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields, Data, DataStruct, DataEnum};

use crate::attrs::{ContainerAttrs, FieldAttrs};
use crate::util;

/// Generate the `collect_schemas()` method body for a struct.
pub fn generate_collect_schemas(input: &DeriveInput) -> syn::Result<TokenStream> {
    let container_attrs = ContainerAttrs::from_attrs(&input.attrs)?;
    let schema_name = container_attrs
        .rename
        .unwrap_or_else(|| input.ident.to_string());

    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            let mut schema_fields = Vec::new();
            let mut nested_schema_calls = Vec::new();

            for field in &fields.named {
                let field_attrs = FieldAttrs::from_attrs(&field.attrs)?;
                if field_attrs.skip {
                    continue;
                }

                let field_name = field_attrs
                    .rename
                    .unwrap_or_else(|| field.ident.as_ref().unwrap().to_string());

                let ty = &field.ty;

                // Flatten: merge nested type's schemas but don't add a field for the flattened member
                if field_attrs.flatten {
                    let effective_ty = if let Some(inner) = util::extract_option_inner(ty) {
                        inner
                    } else {
                        ty
                    };
                    // Collect schemas from the flattened type (includes its own schema)
                    nested_schema_calls.push(quote! {
                        schemas.extend(<#effective_ty as ::tealeaf::convert::ToTeaLeaf>::collect_schemas());
                    });
                    continue;
                }

                // If type override, use that
                if let Some(ref type_str) = field_attrs.type_override {
                    let nullable = field_attrs.optional || util::is_option_type(ty);
                    schema_fields.push(quote! {
                        {
                            let mut ft = ::tealeaf::FieldType::new(#type_str);
                            if #nullable {
                                ft = ft.nullable();
                            }
                            schema.add_field(#field_name, ft);
                        }
                    });
                } else {
                    // Use the type's tealeaf_field_type() method
                    let effective_ty = if let Some(inner) = util::extract_option_inner(ty) {
                        // Option<T> -> T's field type with nullable
                        inner
                    } else {
                        ty
                    };

                    let is_nullable = field_attrs.optional || util::is_option_type(ty);

                    // Collect schemas from nested type
                    nested_schema_calls.push(quote! {
                        schemas.extend(<#effective_ty as ::tealeaf::convert::ToTeaLeaf>::collect_schemas());
                    });

                    schema_fields.push(quote! {
                        {
                            let mut ft = <#effective_ty as ::tealeaf::convert::ToTeaLeaf>::tealeaf_field_type();
                            if #is_nullable {
                                ft = ft.nullable();
                            }
                            schema.add_field(#field_name, ft);
                        }
                    });
                }
            }

            Ok(quote! {
                fn collect_schemas() -> ::tealeaf::IndexMap<String, ::tealeaf::Schema> {
                    let mut schemas = ::tealeaf::IndexMap::new();
                    // Collect schemas from nested types
                    #(#nested_schema_calls)*
                    // Build own schema
                    let mut schema = ::tealeaf::Schema::new(#schema_name);
                    #(#schema_fields)*
                    schemas.insert(#schema_name.to_string(), schema);
                    schemas
                }
            })
        }
        Data::Enum(_) => {
            // Enums don't produce struct schemas, but nested types in variants might
            Ok(quote! {
                fn collect_schemas() -> ::tealeaf::IndexMap<String, ::tealeaf::Schema> {
                    ::tealeaf::IndexMap::new()
                }
            })
        }
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "ToTeaLeaf can only be derived for structs with named fields or enums",
        )),
    }
}

/// Generate the `collect_unions()` method body.
///
/// For enums: builds a `Union` with all variants and their fields.
/// For structs: propagates `collect_unions()` from nested field types.
pub fn generate_collect_unions(input: &DeriveInput) -> syn::Result<TokenStream> {
    let container_attrs = ContainerAttrs::from_attrs(&input.attrs)?;
    let union_name = container_attrs
        .rename
        .unwrap_or_else(|| input.ident.to_string());

    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => {
            // Structs propagate collect_unions() from nested field types
            let mut nested_union_calls = Vec::new();

            for field in &fields.named {
                let field_attrs = FieldAttrs::from_attrs(&field.attrs)?;
                if field_attrs.skip {
                    continue;
                }

                let ty = &field.ty;
                let effective_ty = if let Some(inner) = util::extract_option_inner(ty) {
                    inner
                } else {
                    ty
                };

                nested_union_calls.push(quote! {
                    unions.extend(<#effective_ty as ::tealeaf::convert::ToTeaLeaf>::collect_unions());
                });
            }

            Ok(quote! {
                fn collect_unions() -> ::tealeaf::IndexMap<String, ::tealeaf::Union> {
                    let mut unions = ::tealeaf::IndexMap::new();
                    #(#nested_union_calls)*
                    unions
                }
            })
        }
        Data::Enum(data) => {
            generate_enum_collect_unions(&union_name, data)
        }
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "ToTeaLeaf can only be derived for structs with named fields or enums",
        )),
    }
}

fn generate_enum_collect_unions(union_name: &str, data: &DataEnum) -> syn::Result<TokenStream> {
    let mut variant_builds = Vec::new();

    for variant in &data.variants {
        let variant_name = variant.ident.to_string();

        match &variant.fields {
            Fields::Named(fields) => {
                let mut field_adds = Vec::new();
                for field in &fields.named {
                    let field_attrs = FieldAttrs::from_attrs(&field.attrs)?;
                    if field_attrs.skip { continue; }

                    let fname = field_attrs
                        .rename
                        .unwrap_or_else(|| field.ident.as_ref().unwrap().to_string());
                    let ty = &field.ty;
                    let effective_ty = if let Some(inner) = util::extract_option_inner(ty) {
                        inner
                    } else {
                        ty
                    };
                    let is_nullable = field_attrs.optional || util::is_option_type(ty);

                    if let Some(ref type_str) = field_attrs.type_override {
                        field_adds.push(quote! {
                            {
                                let mut ft = ::tealeaf::FieldType::new(#type_str);
                                if #is_nullable { ft = ft.nullable(); }
                                variant.fields.push(::tealeaf::Field::new(#fname, ft));
                            }
                        });
                    } else {
                        field_adds.push(quote! {
                            {
                                let mut ft = <#effective_ty as ::tealeaf::convert::ToTeaLeaf>::tealeaf_field_type();
                                if #is_nullable { ft = ft.nullable(); }
                                variant.fields.push(::tealeaf::Field::new(#fname, ft));
                            }
                        });
                    }
                }

                variant_builds.push(quote! {
                    {
                        let mut variant = ::tealeaf::Variant::new(#variant_name);
                        #(#field_adds)*
                        union_def.add_variant(variant);
                    }
                });
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() == 1 {
                    // Single unnamed field: variant with one positional field
                    let ty = &fields.unnamed[0].ty;
                    variant_builds.push(quote! {
                        {
                            let variant = ::tealeaf::Variant::new(#variant_name)
                                .field("0", <#ty as ::tealeaf::convert::ToTeaLeaf>::tealeaf_field_type());
                            union_def.add_variant(variant);
                        }
                    });
                } else {
                    // Multiple unnamed fields: variant with positional fields
                    let mut field_adds = Vec::new();
                    for (i, field) in fields.unnamed.iter().enumerate() {
                        let idx_str = i.to_string();
                        let ty = &field.ty;
                        field_adds.push(quote! {
                            variant.fields.push(::tealeaf::Field::new(
                                #idx_str,
                                <#ty as ::tealeaf::convert::ToTeaLeaf>::tealeaf_field_type(),
                            ));
                        });
                    }
                    variant_builds.push(quote! {
                        {
                            let mut variant = ::tealeaf::Variant::new(#variant_name);
                            #(#field_adds)*
                            union_def.add_variant(variant);
                        }
                    });
                }
            }
            Fields::Unit => {
                // Unit variant: no fields
                variant_builds.push(quote! {
                    union_def.add_variant(::tealeaf::Variant::new(#variant_name));
                });
            }
        }
    }

    Ok(quote! {
        fn collect_unions() -> ::tealeaf::IndexMap<String, ::tealeaf::Union> {
            let mut unions = ::tealeaf::IndexMap::new();
            let mut union_def = ::tealeaf::Union::new(#union_name);
            #(#variant_builds)*
            unions.insert(#union_name.to_string(), union_def);
            unions
        }
    })
}
