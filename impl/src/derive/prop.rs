use syn::Type;

use super::ast::Field;
use super::ast::Struct;
use super::ast::Variant;
use super::attr::From;
use super::unraw::MemberUnraw;

/// Shared source, conversion, and backtrace selection for structs and variants.
pub(super) trait FieldContainer {
  /// Borrow every original field in declaration order.
  fn fields(&self) -> &[Field<'_>];

  /// Select a conversion field together with its original attribute.
  fn conversion_field(&self) -> Option<(&Field<'_>, From<'_>)> {
    self
      .fields()
      .iter()
      .find_map(|field| field.attrs.from.map(|attribute| (field, attribute)))
  }

  /// Explicit and conversion-implied sources take precedence over conventional field naming.
  fn source_field(&self) -> Option<&Field<'_>> {
    self
      .fields()
      .iter()
      .find(|field| field.attrs.from.is_some() || field.attrs.source.is_some())
      .or_else(|| {
        self.fields().iter().find(|field| match field.member {
          MemberUnraw::Named(ref ident) => ident == "source",
          MemberUnraw::Unnamed(_) => false,
        })
      })
  }

  /// Explicit providers take precedence over automatic backtrace type recognition.
  fn backtrace_field(&self) -> Option<&Field<'_>> {
    self
      .fields()
      .iter()
      .find(|field| field.attrs.backtrace.is_some())
      .or_else(|| self.fields().iter().find(|field| field.is_backtrace()))
  }

  /// Select a captured backtrace that differs from the conversion source.
  fn distinct_backtrace_field(&self) -> Option<&Field<'_>> {
    let backtrace = self.backtrace_field()?;
    if self
      .conversion_field()
      .is_some_and(|conversion| conversion.0.member == backtrace.member)
    {
      None
    } else {
      Some(backtrace)
    }
  }
}

impl FieldContainer for Struct<'_> {
  fn fields(&self) -> &[Field<'_>] {
    &self.fields
  }
}

impl FieldContainer for Variant<'_> {
  fn fields(&self) -> &[Field<'_>] {
    &self.fields
  }
}

/// Resolve automatic backtrace provision from the parsed final path segment.
#[allow(
  clippy::single_call_fn,
  reason = "automatic backtrace recognition is a syntax rule distinct from explicit field attributes"
)]
pub(super) fn type_is_backtrace(ty: &Type) -> bool {
  let Type::Path(ref syntax) = *ty else {
    return false;
  };
  syntax
    .path
    .segments
    .last()
    .is_some_and(|segment| segment.ident == "Backtrace" && segment.arguments.is_empty())
}
