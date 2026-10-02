use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use super::expand::call_site_ident;
use crate::Private;

/// Emit the original diagnostic and fallible trait stubs to minimize cascading errors.
#[allow(
  clippy::single_call_fn,
  reason = "invalid derives share one diagnostic-preserving fallback expansion"
)]
pub(super) fn expand(input: &DeriveInput, error: &syn::Error) -> TokenStream {
  let ty = call_site_ident(&input.ident);
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let diagnostic = error.to_compile_error();

  quote! {
      #diagnostic

      #[automatically_derived]
      impl #impl_generics ::thiserror::#Private::Error for #ty #ty_generics #where_clause
      where
          // Work around trivial bounds being unstable.
          // https://github.com/rust-lang/rust/issues/48214
          for<'workaround> #ty #ty_generics: ::core::fmt::Debug,
      {}

      #[automatically_derived]
      impl #impl_generics ::core::fmt::Display for #ty #ty_generics #where_clause {
          fn fmt(&self, _: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
              ::core::result::Result::Err(::core::fmt::Error)
          }
      }
  }
}
