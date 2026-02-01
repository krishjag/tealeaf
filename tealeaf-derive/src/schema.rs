//! Schema generation helpers for derive macros.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields, Data, DataStruct};

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
                fn collect_schemas() -> ::std::collections::HashMap<String, ::tealeaf::Schema> {
                    let mut schemas = ::std::collections::HashMap::new();
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
            // Enums handled in Phase 3
            Ok(quote! {
                fn collect_schemas() -> ::std::collections::HashMap<String, ::tealeaf::Schema> {
                    ::std::collections::HashMap::new()
                }
            })
        }
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "ToTeaLeaf can only be derived for structs with named fields or enums",
        )),
    }
}
