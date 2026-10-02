//! Parse, validate, and derive typed errors for the `thiserror` runtime.

/// Internal syntax, validation, inference, and trait-emission engine.
mod derive;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::Span;
use quote::ToTokens;
use quote::TokenStreamExt as _;
use syn::DeriveInput;
use syn::parse_macro_input;

/// Expand the complete native declaration through the internal derive engine.
trait ErrorExpansion {
  /// Return generated implementations or the declaration's primary compiler diagnostic.
  fn expand_error(&self) -> proc_macro2::TokenStream;
}

/// Derive `Error`, optional `Display`, and requested `From` implementations.
#[allow(
  clippy::single_call_fn,
  reason = "rustc registers this procedural-macro entry point rather than calling it through Rust source"
)]
#[proc_macro_derive(Error, attributes(backtrace, error, from, source))]
pub fn derive_error(input: TokenStream) -> TokenStream {
  let declaration = parse_macro_input!(input as DeriveInput);
  declaration.expand_error().into()
}

/// Emit the versioned runtime namespace shared with the root crate's generated module.
struct Private;

impl ToTokens for Private {
  fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
    tokens.append(Ident::new(concat!("__private", env!("CARGO_PKG_VERSION_PATCH")), Span::call_site()));
  }
}
