//! Derive macros for TeaLeaf DTO conversion.
//!
//! Provides `#[derive(ToTeaLeaf)]` and `#[derive(FromTeaLeaf)]` for automatic
//! conversion between Rust structs/enums and TeaLeaf `Value` types.

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod attrs;
mod from_tealeaf;
mod schema;
mod to_tealeaf;
mod util;

/// Derive `ToTeaLeaf` for a struct or enum.
///
/// # Example
///
/// ```ignore
/// use tealeaf::ToTeaLeaf;
///
/// #[derive(ToTeaLeaf)]
/// struct User {
///     id: i64,
///     name: String,
///     #[tealeaf(optional)]
///     email: Option<String>,
/// }
/// ```
#[proc_macro_derive(ToTeaLeaf, attributes(tealeaf))]
pub fn derive_to_tealeaf(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    to_tealeaf::derive(&input)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}

/// Derive `FromTeaLeaf` for a struct or enum.
///
/// # Example
///
/// ```ignore
/// use tealeaf::FromTeaLeaf;
///
/// #[derive(FromTeaLeaf)]
/// struct User {
///     id: i64,
///     name: String,
///     #[tealeaf(optional)]
///     email: Option<String>,
/// }
/// ```
#[proc_macro_derive(FromTeaLeaf, attributes(tealeaf))]
pub fn derive_from_tealeaf(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    from_tealeaf::derive(&input)
        .unwrap_or_else(|e| e.into_compile_error())
        .into()
}
