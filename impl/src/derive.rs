//! Internal owner of parsed error declarations and their generated standard contracts.

/// Borrowed error declarations and field structure.
mod ast;
/// Error attributes and formatting contracts.
mod attr;
/// Emit source, display, conversion, and provider implementations.
mod expand;
/// Preserve primary diagnostics after an invalid declaration.
mod fallback;
/// Interpret format shorthand and retain opaque expressions.
mod fmt;
/// Infer complete non-recursive field predicates.
mod generics;
/// Select concrete sources, conversions, and backtraces.
mod prop;
/// Preserve identifier hygiene and raw field spelling.
mod unraw;
/// Validate complete declarations before trait emission.
mod valid;

impl crate::ErrorExpansion for syn::DeriveInput {
  fn expand_error(&self) -> proc_macro2::TokenStream {
    match expand::try_expand(self) {
      Ok(expanded) => expanded,
      Err(error) => fallback::expand(self, &error),
    }
  }
}
