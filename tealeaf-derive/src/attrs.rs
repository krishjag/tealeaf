//! Attribute parsing for `#[tealeaf(...)]` annotations.

use syn::{Attribute, Lit};

/// Container-level attributes (on the struct/enum itself)
#[derive(Debug, Default)]
pub struct ContainerAttrs {
    /// Override the schema name: `#[tealeaf(rename = "my_name")]`
    pub rename: Option<String>,
    /// Mark as root-level array: `#[tealeaf(root_array)]`
    pub root_array: bool,
    /// Custom data key: `#[tealeaf(key = "my_key")]`
    pub key: Option<String>,
}

/// Field-level attributes
#[derive(Debug, Default)]
pub struct FieldAttrs {
    /// Override the field name: `#[tealeaf(rename = "field_name")]`
    pub rename: Option<String>,
    /// Skip this field: `#[tealeaf(skip)]`
    pub skip: bool,
    /// Mark as optional/nullable: `#[tealeaf(optional)]`
    pub optional: bool,
    /// Override the TeaLeaf type: `#[tealeaf(type = "timestamp")]`
    pub type_override: Option<String>,
    /// Flatten nested struct: `#[tealeaf(flatten)]`
    pub flatten: bool,
    /// Use Default::default() when deserializing missing field: `#[tealeaf(default)]`
    pub default: bool,
    /// Custom default expression: `#[tealeaf(default = "expr")]`
    pub default_expr: Option<String>,
}

impl ContainerAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();
        for attr in attrs {
            if !attr.path().is_ident("tealeaf") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        result.rename = Some(s.value());
                    }
                    return Ok(());
                }
                if meta.path.is_ident("root_array") {
                    result.root_array = true;
                    return Ok(());
                }
                if meta.path.is_ident("key") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        result.key = Some(s.value());
                    }
                    return Ok(());
                }
                Err(meta.error("unknown tealeaf container attribute"))
            })?;
        }
        Ok(result)
    }
}

impl FieldAttrs {
    pub fn from_attrs(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();
        for attr in attrs {
            if !attr.path().is_ident("tealeaf") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        result.rename = Some(s.value());
                    }
                    return Ok(());
                }
                if meta.path.is_ident("skip") {
                    result.skip = true;
                    return Ok(());
                }
                if meta.path.is_ident("optional") {
                    result.optional = true;
                    return Ok(());
                }
                if meta.path.is_ident("flatten") {
                    result.flatten = true;
                    return Ok(());
                }
                if meta.path.is_ident("default") {
                    // Check if it has a value (default = "expr") or is bare (default)
                    if meta.input.peek(syn::Token![=]) {
                        let value = meta.value()?;
                        let lit: Lit = value.parse()?;
                        if let Lit::Str(s) = lit {
                            result.default_expr = Some(s.value());
                        }
                    }
                    result.default = true;
                    return Ok(());
                }
                if meta.path.is_ident("type") {
                    let value = meta.value()?;
                    let lit: Lit = value.parse()?;
                    if let Lit::Str(s) = lit {
                        result.type_override = Some(s.value());
                    }
                    return Ok(());
                }
                Err(meta.error("unknown tealeaf field attribute"))
            })?;
        }
        Ok(result)
    }
}
