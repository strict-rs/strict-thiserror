use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FormatResult;

use proc_macro2::Span;
use syn::Data;
use syn::DataEnum;
use syn::DataStruct;
use syn::DeriveInput;
use syn::Error;
use syn::Fields;
use syn::Generics;
use syn::Ident;
use syn::Index;
use syn::Result;
use syn::Type;

use super::attr;
use super::attr::Attrs;
use super::generics::ParamsInScope;
use super::prop::FieldContainer as _;
use super::prop::type_is_backtrace;
use super::unraw::IdentUnraw;
use super::unraw::MemberUnraw;

/// Error declaration supported by the derive.
pub(super) enum Input<'a> {
  /// One struct error with named, unnamed, or no fields.
  Struct(Struct<'a>),
  /// An enum whose variants independently select display and source behavior.
  Enum(Enum<'a>),
}

/// Borrowed struct syntax with resolved derive attributes.
pub(super) struct Struct<'a> {
  /// Attributes on the struct declaration.
  pub(super) attrs:    Attrs<'a>,
  /// Struct name with its original span.
  pub(super) ident:    Ident,
  /// Original parameters and user-authored bounds.
  pub(super) generics: &'a Generics,
  /// Fields in declaration order.
  pub(super) fields:   Vec<Field<'a>>,
}

/// Borrowed enum syntax with resolved variant attributes.
pub(super) struct Enum<'a> {
  /// Attributes inherited by variants without a local display form.
  pub(super) attrs:    Attrs<'a>,
  /// Enum name with its original span.
  pub(super) ident:    Ident,
  /// Original parameters and user-authored bounds.
  pub(super) generics: &'a Generics,
  /// Variants in declaration order.
  pub(super) variants: Vec<Variant<'a>>,
}

/// Enum variant with locally selected or inherited error attributes.
pub(super) struct Variant<'a> {
  /// Original variant retained for complete diagnostic spans.
  pub(super) original: &'a syn::Variant,
  /// Resolved local or inherited attributes.
  pub(super) attrs:    Attrs<'a>,
  /// Variant name with its original span.
  pub(super) ident:    Ident,
  /// Fields in declaration order.
  pub(super) fields:   Vec<Field<'a>>,
}

/// Field syntax, derive attributes, and its inferred-bound eligibility.
pub(super) struct Field<'a> {
  /// Original field retained for type and attribute diagnostics.
  pub(super) original:              &'a syn::Field,
  /// Explicit source, conversion, and backtrace attributes.
  pub(super) attrs:                 Attrs<'a>,
  /// Named or numbered field access with raw identifier normalization.
  pub(super) member:                MemberUnraw,
  /// Complete original field type.
  pub(super) ty:                    &'a Type,
  /// Whether whole-field bounds can be inferred without requiring a recursive implementation.
  pub(super) needs_inferred_bounds: bool,
}

/// Container shape used to explain positional formatting ambiguity.
#[derive(Copy, Clone)]
pub(super) enum ContainerKind {
  /// Struct with named fields.
  Struct,
  /// Struct with numbered fields.
  TupleStruct,
  /// Struct without fields.
  UnitStruct,
  /// Enum variant with named fields.
  StructVariant,
  /// Enum variant with numbered fields.
  TupleVariant,
  /// Enum variant without fields.
  UnitVariant,
}

impl<'a> Input<'a> {
  /// Resolve attributes and field shorthand for the supported error declaration.
  ///
  /// # Errors
  /// Rejects unions, malformed attributes, invalid shorthand, or an unrepresentable field index.
  #[allow(
    clippy::single_call_fn,
    reason = "declaration parsing establishes the complete native input model before validation and emission"
  )]
  pub(super) fn from_syn(node: &'a DeriveInput) -> Result<Self> {
    match node.data {
      Data::Struct(ref struct_data) => Struct::from_syn(node, struct_data).map(Input::Struct),
      Data::Enum(ref enum_data) => Enum::from_syn(node, enum_data).map(Input::Enum),
      Data::Union(_) => Err(Error::new_spanned(node, "union as errors are not supported")),
    }
  }
}

impl<'a> Struct<'a> {
  /// Resolve struct attributes before interpreting its fields and formatting shorthand.
  #[allow(
    clippy::single_call_fn,
    reason = "struct parsing resolves declaration-wide formatting context before constructing the shared field model"
  )]
  fn from_syn(node: &'a DeriveInput, struct_data: &'a DataStruct) -> Result<Self> {
    let mut attrs = attr::get(&node.attrs)?;
    let scope = ParamsInScope::new(&node.generics);
    let fields = Field::multiple_from_syn(&struct_data.fields, &scope)?;
    if let Some(display) = attrs.display.as_mut() {
      let container = ContainerKind::from_struct(struct_data);
      display.expand_shorthand(&fields, container)?;
    }
    Ok(Struct {
      attrs,
      ident: node.ident.clone(),
      generics: &node.generics,
      fields,
    })
  }
}

impl<'a> Enum<'a> {
  /// Whether any variant exposes a source or delegates to a transparent error.
  pub(super) fn has_source(&self) -> bool {
    self
      .variants
      .iter()
      .any(|variant| variant.source_field().is_some() || variant.attrs.transparent.is_some())
  }

  /// Whether any variant provides a backtrace.
  pub(super) fn has_backtrace(&self) -> bool {
    self.variants.iter().any(|variant| variant.backtrace_field().is_some())
  }

  /// Whether all emitted variants have a supported display form.
  pub(super) fn has_display(&self) -> bool {
    self.attrs.display.is_some()
      || self.attrs.transparent.is_some()
      || self.attrs.fmt.is_some()
      || self
        .variants
        .iter()
        .any(|variant| variant.attrs.display.is_some() || variant.attrs.fmt.is_some())
      || self.variants.iter().all(|variant| variant.attrs.transparent.is_some())
  }

  /// Resolve enum-level inheritance and each variant's formatting context.
  #[allow(
    clippy::single_call_fn,
    reason = "enum parsing owns attribute inheritance and variant-local formatting contexts"
  )]
  fn from_syn(node: &'a DeriveInput, enum_data: &'a DataEnum) -> Result<Self> {
    let attrs = attr::get(&node.attrs)?;
    let scope = ParamsInScope::new(&node.generics);
    let variants = enum_data
      .variants
      .iter()
      .map(|variant_syntax| {
        let mut variant = Variant::from_syn(variant_syntax, &scope)?;
        if variant.attrs.display.is_none() && variant.attrs.transparent.is_none() && variant.attrs.fmt.is_none() {
          variant.attrs.display.clone_from(&attrs.display);
          variant.attrs.transparent = attrs.transparent;
          variant.attrs.fmt.clone_from(&attrs.fmt);
        }
        if let Some(display) = variant.attrs.display.as_mut() {
          let container = ContainerKind::from_variant(variant_syntax);
          display.expand_shorthand(&variant.fields, container)?;
        }
        Ok(variant)
      })
      .collect::<Result<_>>()?;
    Ok(Enum {
      attrs,
      ident: node.ident.clone(),
      generics: &node.generics,
      variants,
    })
  }
}

impl<'a> Variant<'a> {
  /// Resolve a variant's original attributes and fields before enum-level inheritance.
  #[allow(
    clippy::single_call_fn,
    reason = "variant parsing supplies the complete local model used by enum attribute inheritance"
  )]
  fn from_syn(node: &'a syn::Variant, scope: &ParamsInScope<'a>) -> Result<Self> {
    let attrs = attr::get(&node.attrs)?;
    Ok(Variant {
      original: node,
      attrs,
      ident: node.ident.clone(),
      fields: Field::multiple_from_syn(&node.fields, scope)?,
    })
  }
}

impl<'a> Field<'a> {
  /// Whether the field type selects automatic backtrace provision.
  pub(super) fn is_backtrace(&self) -> bool {
    type_is_backtrace(self.ty)
  }

  /// Select the original explicit or conversion-implied source span.
  pub(super) fn source_span(&self) -> Span {
    self
      .attrs
      .source
      .as_ref()
      .map(|attribute| attribute.span)
      .or_else(|| self.attrs.from.as_ref().map(|attribute| attribute.span))
      .unwrap_or_else(|| self.member.span())
  }

  /// Resolve each field in declaration order with its original parameter scope.
  fn multiple_from_syn(fields: &'a Fields, scope: &ParamsInScope<'a>) -> Result<Vec<Self>> {
    fields
      .iter()
      .enumerate()
      .map(|(i, field)| Field::from_syn(i, field, scope))
      .collect()
  }

  /// Resolve attributes, member access, and non-recursive inferred-bound eligibility.
  #[allow(
    clippy::single_call_fn,
    reason = "field parsing combines syntax, attributes, member identity, and inference eligibility in one checked construction"
  )]
  fn from_syn(i: usize, node: &'a syn::Field, scope: &ParamsInScope<'a>) -> Result<Self> {
    Ok(Field {
      original:              node,
      attrs:                 attr::get(&node.attrs)?,
      member:                match node.ident {
        Some(ref name) => MemberUnraw::Named(IdentUnraw::new(name.clone())),
        None => MemberUnraw::Unnamed(Index {
          index: u32::try_from(i).map_err(|source| Error::new_spanned(node, source))?,
          span:  Span::call_site(),
        }),
      },
      ty:                    &node.ty,
      needs_inferred_bounds: scope.needs_inferred_bounds(&node.ty),
    })
  }
}

impl ContainerKind {
  /// Classify a struct using its original named, numbered, or unit field form.
  #[allow(
    clippy::single_call_fn,
    reason = "struct field shape selects positional-argument diagnostics independently of variant parsing"
  )]
  const fn from_struct(node: &DataStruct) -> Self {
    match node.fields {
      Fields::Named(_) => Self::Struct,
      Fields::Unnamed(_) => Self::TupleStruct,
      Fields::Unit => Self::UnitStruct,
    }
  }

  /// Classify an enum variant using its original field form.
  #[allow(
    clippy::single_call_fn,
    reason = "variant field shape selects the corresponding positional-argument diagnostic"
  )]
  const fn from_variant(node: &syn::Variant) -> Self {
    match node.fields {
      Fields::Named(_) => Self::StructVariant,
      Fields::Unnamed(_) => Self::TupleVariant,
      Fields::Unit => Self::UnitVariant,
    }
  }
}

impl Display for ContainerKind {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
    formatter.write_str(match *self {
      Self::Struct => "struct",
      Self::TupleStruct => "tuple struct",
      Self::UnitStruct => "unit struct",
      Self::StructVariant => "struct variant",
      Self::TupleVariant => "tuple variant",
      Self::UnitVariant => "unit variant",
    })
  }
}
