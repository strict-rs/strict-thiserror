use syn::Error;
use syn::GenericArgument;
use syn::PathArguments;
use syn::Result;
use syn::Type;

use super::ast::Enum;
use super::ast::Field;
use super::ast::Input;
use super::ast::Struct;
use super::ast::Variant;
use super::attr::Attrs;

/// Validate a complete syntax owner before emitting standard error implementations.
pub(super) trait Validate {
  /// Reject invalid attributes, field relationships, or concrete source lifetimes.
  ///
  /// # Errors
  /// Returns the original declaration, attribute, or field diagnostic.
  fn validate(&self) -> Result<()>;
}

impl Validate for Input<'_> {
  /// Validate attribute placement, transparent fields, and concrete source lifetimes.
  ///
  /// # Errors
  /// Returns the original attribute or field diagnostic when the declaration violates its contract.
  fn validate(&self) -> Result<()> {
    match *self {
      Input::Struct(ref input) => input.validate(),
      Input::Enum(ref input) => input.validate(),
    }
  }
}

impl Validate for Struct<'_> {
  /// Validate this syntax owner before generating any standard trait implementation.
  fn validate(&self) -> Result<()> {
    check_non_field_attrs(&self.attrs)?;
    if let Some(transparent) = self.attrs.transparent {
      if self.fields.len() != 1 {
        return Err(Error::new_spanned(
          transparent.original,
          "#[error(transparent)] requires exactly one field",
        ));
      }
      if let Some(source) = self.fields.iter().find_map(|f| f.attrs.source) {
        return Err(Error::new_spanned(
          source.original,
          "transparent error struct can't contain #[source]",
        ));
      }
    }
    if let Some(fmt) = self.attrs.fmt.as_ref() {
      return Err(Error::new_spanned(
        fmt.original,
        "#[error(fmt = ...)] is only supported in enums; for a struct, handwrite your own Display impl",
      ));
    }
    check_field_attrs(&self.fields)?;
    for field in &self.fields {
      field.validate()?;
    }
    Ok(())
  }
}

impl Validate for Enum<'_> {
  /// Validate this syntax owner before generating any standard trait implementation.
  fn validate(&self) -> Result<()> {
    check_non_field_attrs(&self.attrs)?;
    let has_display = self.has_display();
    for variant in &self.variants {
      variant.validate()?;
      if has_display && variant.attrs.display.is_none() && variant.attrs.transparent.is_none() && variant.attrs.fmt.is_none() {
        return Err(Error::new_spanned(variant.original, "missing #[error(\"...\")] display attribute"));
      }
    }
    Ok(())
  }
}

impl Validate for Variant<'_> {
  /// Validate this syntax owner before generating any standard trait implementation.
  fn validate(&self) -> Result<()> {
    check_non_field_attrs(&self.attrs)?;
    if self.attrs.transparent.is_some() {
      if self.fields.len() != 1 {
        return Err(Error::new_spanned(
          self.original,
          "#[error(transparent)] requires exactly one field",
        ));
      }
      if let Some(source) = self.fields.iter().find_map(|f| f.attrs.source) {
        return Err(Error::new_spanned(source.original, "transparent variant can't contain #[source]"));
      }
    }
    check_field_attrs(&self.fields)?;
    for field in &self.fields {
      field.validate()?;
    }
    Ok(())
  }
}

impl Validate for Field<'_> {
  /// Validate this syntax owner before generating any standard trait implementation.
  fn validate(&self) -> Result<()> {
    if let Some(unexpected_display_attr) = self
      .attrs
      .display
      .as_ref()
      .map(|display| display.original)
      .or_else(|| self.attrs.fmt.as_ref().map(|formatter| formatter.original))
    {
      return Err(Error::new_spanned(
        unexpected_display_attr,
        "not expected here; the #[error(...)] attribute belongs on top of a struct or an enum variant",
      ));
    }
    Ok(())
  }
}

/// Reject field-only attributes on declarations and mutually exclusive display forms.
fn check_non_field_attrs(attrs: &Attrs<'_>) -> Result<()> {
  if let Some(from) = attrs.from.as_ref() {
    return Err(Error::new_spanned(
      from.original,
      "not expected here; the #[from] attribute belongs on a specific field",
    ));
  }
  if let Some(source) = attrs.source.as_ref() {
    return Err(Error::new_spanned(
      source.original,
      "not expected here; the #[source] attribute belongs on a specific field",
    ));
  }
  if let Some(backtrace) = attrs.backtrace.as_ref() {
    return Err(Error::new_spanned(
      backtrace,
      "not expected here; the #[backtrace] attribute belongs on a specific field",
    ));
  }
  if attrs.transparent.is_some() {
    if let Some(display) = attrs.display.as_ref() {
      return Err(Error::new_spanned(
        display.original,
        "cannot have both #[error(transparent)] and a display attribute",
      ));
    }
    if let Some(fmt) = attrs.fmt.as_ref() {
      return Err(Error::new_spanned(
        fmt.original,
        "cannot have both #[error(transparent)] and #[error(fmt = ...)]",
      ));
    }
  }
  if attrs.transparent.is_none()
    && let (Some(display), Some(_)) = (attrs.display.as_ref(), attrs.fmt.as_ref())
  {
    return Err(Error::new_spanned(
      display.original,
      "cannot have both #[error(fmt = ...)] and a format arguments attribute",
    ));
  }

  Ok(())
}

/// Validate source uniqueness, conversion ownership, and backtrace field cardinality.
fn check_field_attrs(fields: &[Field<'_>]) -> Result<()> {
  let mut from = None;
  let mut source_field = None;
  let mut backtrace_field = None;
  let mut has_backtrace = false;
  for field in fields {
    if let Some(from_attr) = field.attrs.from {
      if from.is_some() {
        return Err(Error::new_spanned(from_attr.original, "duplicate #[from] attribute"));
      }
      from = Some((field, from_attr));
    }
    if let Some(source) = field.attrs.source {
      if source_field.is_some() {
        return Err(Error::new_spanned(source.original, "duplicate #[source] attribute"));
      }
      source_field = Some(field);
    }
    if let Some(backtrace) = field.attrs.backtrace {
      if backtrace_field.is_some() {
        return Err(Error::new_spanned(backtrace, "duplicate #[backtrace] attribute"));
      }
      backtrace_field = Some(field);
      has_backtrace = true;
    }
    if let Some(transparent) = field.attrs.transparent {
      return Err(Error::new_spanned(
        transparent.original,
        "#[error(transparent)] needs to go outside the enum or struct, not on an individual field",
      ));
    }
    has_backtrace |= field.is_backtrace();
  }
  if let (Some((from_field, from_attr)), Some(explicit_source)) = (from, source_field)
    && from_field.member != explicit_source.member
  {
    return Err(Error::new_spanned(
      from_attr.original,
      "#[from] is only supported on the source field, not any other field",
    ));
  }
  if let Some((from_field, from_attr)) = from {
    let max_expected_fields = match backtrace_field {
      Some(captured_backtrace) if from_field.member != captured_backtrace.member => 2,
      None if has_backtrace => 2,
      Some(_) | None => 1,
    };
    if fields.len() > max_expected_fields {
      return Err(Error::new_spanned(
        from_attr.original,
        "deriving From requires no fields other than source and backtrace",
      ));
    }
  }
  let effective_source = source_field.or_else(|| from.map(|(field, _attribute)| field));
  if let Some(selected_source) = effective_source
    && contains_non_static_lifetime(selected_source.ty)
  {
    return Err(Error::new_spanned(
      &selected_source.original.ty,
      "non-static lifetimes are not allowed in the source of an error, because std::error::Error requires the source is dyn Error + \
       'static",
    ));
  }
  Ok(())
}

/// Recognize non-static source borrows in supported reference and generic path types.
#[allow(
  clippy::single_call_fn,
  reason = "source lifetime validation is independent of attribute placement and conversion field cardinality"
)]
fn contains_non_static_lifetime(ty: &Type) -> bool {
  if let Type::Reference(ref reference) = *ty {
    return reference.lifetime.as_ref().is_some_and(|lifetime| lifetime.ident != "static");
  }
  let Type::Path(ref syntax) = *ty else {
    return false;
  };
  let Some(segment) = syntax.path.segments.last() else {
    return false;
  };
  let PathArguments::AngleBracketed(ref arguments) = segment.arguments else {
    return false;
  };
  arguments.args.iter().any(|argument| {
    if let GenericArgument::Type(ref nested) = *argument {
      return contains_non_static_lifetime(nested);
    }
    if let GenericArgument::Lifetime(ref lifetime) = *argument {
      return lifetime.ident != "static";
    }
    false
  })
}
