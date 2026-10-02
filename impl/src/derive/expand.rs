use std::collections::BTreeSet;

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens as _;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use syn::DeriveInput;
use syn::GenericArgument;
use syn::PathArguments;
use syn::Result;
use syn::Type;

use super::ast::Enum;
use super::ast::Field;
use super::ast::Input;
use super::ast::Struct;
use super::ast::Variant;
use super::attr::Trait;
use super::generics::InferredBounds;
use super::prop::FieldContainer as _;
use super::unraw::IdentUnraw;
use super::unraw::MemberUnraw;
use super::valid::Validate as _;
use crate::Private;

/// Parse and validate the complete declaration before generating any trait implementation.
///
/// # Errors
/// Returns the original parsing, validation, or generated-bound syntax failure.
#[allow(
  clippy::single_call_fn,
  reason = "all generated implementations require the same complete parsing and validation barrier"
)]
pub(super) fn try_expand(declaration: &DeriveInput) -> Result<TokenStream> {
  let parsed = Input::from_syn(declaration)?;
  parsed.validate()?;
  match parsed {
    Input::Struct(structure) => impl_struct(&structure),
    Input::Enum(enumeration) => impl_enum(&enumeration),
  }
}

/// Compose a struct's source, provider, display, and conversion contracts.
///
/// # Errors
/// Returns a malformed generated bound or a missing required transparent field.
#[allow(
  clippy::single_call_fn,
  reason = "struct composition coordinates its four standard trait contracts and shared inferred bounds"
)]
fn impl_struct(input: &Struct<'_>) -> Result<TokenStream> {
  let mut bounds = InferredBounds::new();
  let source = struct_source(input, &mut bounds)?;
  let provide = struct_provide(input);
  let display = struct_display(input)?;
  let from = input.conversion_field().map(|(field, attribute)| {
    from_impl(
      &input.ident,
      input.generics,
      None,
      field,
      input.distinct_backtrace_field(),
      attribute.span,
    )
  });
  let error = error_impl(&input.ident, input.generics, &mut bounds, source.as_ref(), provide.as_ref())?;
  Ok(quote!(#error #display #from))
}

/// Compose an enum's standard contracts without recording bounds for absent methods.
///
/// # Errors
/// Returns malformed generated bounds or a missing transparent variant field.
#[allow(
  clippy::single_call_fn,
  reason = "enum composition preserves the guard that makes source and display arm construction lazy"
)]
fn impl_enum(input: &Enum<'_>) -> Result<TokenStream> {
  let mut bounds = InferredBounds::new();
  let source = if input.has_source() {
    let arms = input
      .variants
      .iter()
      .map(|variant| enum_source_arm(variant, &mut bounds))
      .collect::<Result<Vec<_>>>()?;
    Some(quote! {
      fn source(&self) -> ::core::option::Option<&(dyn ::thiserror::#Private::Error + 'static)> {
        use ::thiserror::#Private::AsDynError as _;
        match self { #(#arms)* }
      }
    })
  } else {
    None
  };
  let provide = input.has_backtrace().then(|| {
    let arms = input.variants.iter().map(enum_provide_arm);
    quote! {
      fn provide<'request>(&'request self, request: &mut ::core::error::Request<'request>) {
        match self { #(#arms)* }
      }
    }
  });
  let display = enum_display(input)?;
  let from = input.variants.iter().filter_map(|variant| {
    variant.conversion_field().map(|(field, attribute)| {
      from_impl(
        &input.ident,
        input.generics,
        Some(&variant.ident),
        field,
        variant.distinct_backtrace_field(),
        attribute.span,
      )
    })
  });
  let error = error_impl(&input.ident, input.generics, &mut bounds, source.as_ref(), provide.as_ref())?;
  Ok(quote!(#error #display #(#from)*))
}

/// Record a field's complete source contract only when that contract is non-recursive.
///
/// # Errors
/// Returns invalid generated trait or lifetime bound syntax.
fn source_bound(field: &Field<'_>, bounds: &mut InferredBounds, transparent: bool) -> Result<()> {
  if field.needs_inferred_bounds {
    let (ty, bound) = if transparent {
      (field.ty, quote!(::thiserror::#Private::Error))
    } else {
      (unoptional_type(field.ty), quote!(::thiserror::#Private::Error + 'static))
    };
    bounds.insert(ty, bound)?;
  }
  Ok(())
}

/// Generate a struct's explicit or transparently delegated source.
///
/// # Errors
/// Returns an invalid inferred bound or the transparent-field validation diagnostic.
#[allow(
  clippy::single_call_fn,
  reason = "struct source emission owns transparent delegation and optional concrete source borrowing"
)]
fn struct_source(input: &Struct<'_>, bounds: &mut InferredBounds) -> Result<Option<TokenStream>> {
  let body = if let Some(transparent) = input.attrs.transparent.as_ref() {
    let field = input
      .fields
      .first()
      .ok_or_else(|| syn::Error::new_spanned(transparent.original, "transparent error requires exactly one field"))?;
    source_bound(field, bounds, true)?;
    let member = &field.member;
    quote_spanned! {transparent.span=>
      ::thiserror::#Private::Error::source(self.#member.as_dyn_error())
    }
  } else if let Some(field) = input.source_field() {
    source_bound(field, bounds, false)?;
    let member = &field.member;
    let asref = type_is_option(field.ty).then(|| quote_spanned!(member.span()=> .as_ref()?));
    let source = quote_spanned!(field.source_span()=> self.#member #asref.as_dyn_error());
    quote!(::core::option::Option::Some(#source))
  } else {
    return Ok(None);
  };
  Ok(Some(quote! {
    fn source(&self) -> ::core::option::Option<&(dyn ::thiserror::#Private::Error + 'static)> {
      use ::thiserror::#Private::AsDynError as _;
      #body
    }
  }))
}

/// Generate one enum source arm, preserving its exact concrete or transparent error type.
///
/// # Errors
/// Returns an invalid bound or a transparent variant without its required field.
#[allow(
  clippy::single_call_fn,
  reason = "source arms distinguish transparent, explicit, optional, and absent variant sources"
)]
fn enum_source_arm(variant: &Variant<'_>, bounds: &mut InferredBounds) -> Result<TokenStream> {
  let name = &variant.ident;
  if let Some(transparent) = variant.attrs.transparent.as_ref() {
    let field = variant
      .fields
      .first()
      .ok_or_else(|| syn::Error::new_spanned(transparent.original, "transparent error requires exactly one field"))?;
    source_bound(field, bounds, true)?;
    let member = &field.member;
    let variable = quote!(transparent);
    let body = quote_spanned!(transparent.span=> ::thiserror::#Private::Error::source(#variable.as_dyn_error()));
    Ok(quote!(Self::#name { #member: #variable } => #body,))
  } else if let Some(field) = variant.source_field() {
    source_bound(field, bounds, false)?;
    let member = &field.member;
    let asref = type_is_option(field.ty).then(|| quote_spanned!(member.span()=> .as_ref()?));
    let variable = quote!(source);
    let source = quote_spanned!(field.source_span()=> #variable #asref.as_dyn_error());
    Ok(quote!(Self::#name { #member: #variable, .. } => ::core::option::Option::Some(#source),))
  } else {
    Ok(quote!(Self::#name { .. } => ::core::option::Option::None,))
  }
}

/// Generate struct backtrace registration and source request forwarding.
#[allow(
  clippy::single_call_fn,
  reason = "struct request emission coordinates shared-source and independently captured backtraces"
)]
fn struct_provide(input: &Struct<'_>) -> Option<TokenStream> {
  let field = input.backtrace_field()?;
  let backtrace = &field.member;
  let request = quote!(request);
  let own_body = if type_is_option(field.ty) {
    quote! {
      if let ::core::option::Option::Some(backtrace) = &self.#backtrace {
        request.provide_ref::<::thiserror::#Private::Backtrace>(backtrace);
      }
    }
  } else {
    quote!(request.provide_ref::<::thiserror::#Private::Backtrace>(&self.#backtrace);)
  };
  let body = input.source_field().map_or_else(
    || own_body.clone(),
    |source| {
      let source_member = &source.member;
      let source_body = source_provide_body(source, &quote!(self.#source_member), source_member.span(), &request);
      let independent = (source_member != backtrace).then_some(&own_body);
      quote! {
        use ::thiserror::#Private::ThiserrorProvide as _;
        #source_body #independent
      }
    },
  );
  Some(quote! {
    fn provide<'request>(&'request self, request: &mut ::core::error::Request<'request>) {
      #body
    }
  })
}

/// Generate one enum provider arm without invoking providers for absent backtraces.
#[allow(
  clippy::single_call_fn,
  reason = "provider arms preserve shared and independent source/backtrace relationships"
)]
fn enum_provide_arm(variant: &Variant<'_>) -> TokenStream {
  let name = &variant.ident;
  let request = quote!(request);
  match (variant.backtrace_field(), variant.source_field()) {
    (Some(backtrace_field), Some(source_field)) if backtrace_field.attrs.backtrace.is_none() => {
      let backtrace = &backtrace_field.member;
      let source = &source_field.member;
      let source_body = source_provide_body(source_field, &quote!(source), source.span(), &request);
      let own_body = backtrace_provide_body(backtrace_field, &request);
      quote! {
        Self::#name { #backtrace: backtrace, #source: source, .. } => {
          use ::thiserror::#Private::ThiserrorProvide as _;
          #source_body #own_body
        }
      }
    }
    (Some(backtrace_field), Some(source_field)) if backtrace_field.member == source_field.member => {
      let member = &backtrace_field.member;
      let body = source_provide_body(source_field, &quote!(source), member.span(), &request);
      quote! {
        Self::#name { #member: source, .. } => {
          use ::thiserror::#Private::ThiserrorProvide as _;
          #body
        }
      }
    }
    (Some(backtrace_field), Some(_) | None) => {
      let member = &backtrace_field.member;
      let body = backtrace_provide_body(backtrace_field, &request);
      quote!(Self::#name { #member: backtrace, .. } => { #body })
    }
    (None, Some(_) | None) => quote!(Self::#name { .. } => {}),
  }
}

/// Generate a struct's literal, formatted, or transparent display implementation.
///
/// # Errors
/// Returns an invalid generated bound or required transparent field.
#[allow(
  clippy::single_call_fn,
  reason = "struct display emission selects one of the supported formatting contracts"
)]
fn struct_display(input: &Struct<'_>) -> Result<Option<TokenStream>> {
  let formatter = Ident::new("formatter", Span::mixed_site());
  let mut bounds = InferredBounds::new();
  let body = if let Some(transparent) = input.attrs.transparent.as_ref() {
    let field = input
      .fields
      .first()
      .ok_or_else(|| syn::Error::new_spanned(transparent.original, "transparent error requires exactly one field"))?;
    if field.needs_inferred_bounds {
      bounds.insert(field.ty, Trait::Display.to_token_stream())?;
    }
    let member = &field.member;
    quote!(::core::fmt::Display::fmt(&self.#member, #formatter))
  } else if let Some(display) = input.attrs.display.as_ref() {
    display_bounds(&input.fields, &display.implied_bounds, &mut bounds)?;
    let imports = use_as_display(display.uses_display_view);
    let referenced = display_references(display);
    let pattern = fields_pat(&input.fields, Some(&referenced));
    let message = display.render(&formatter);
    quote! {
      #imports
      let Self #pattern = self;
      #message
    }
  } else {
    return Ok(None);
  };
  Ok(Some(display_impl(&input.ident, input.generics, &bounds, &formatter, &body)))
}

/// Generate an enum's complete display match after resolving all variant bounds.
///
/// # Errors
/// Returns invalid generated bounds or missing required transparent fields.
#[allow(
  clippy::single_call_fn,
  reason = "enum display composition preserves complete variant coverage and inferred bounds"
)]
fn enum_display(input: &Enum<'_>) -> Result<Option<TokenStream>> {
  if !input.has_display() {
    return Ok(None);
  }
  let formatter = Ident::new("formatter", Span::mixed_site());
  let mut bounds = InferredBounds::new();
  let arms = input
    .variants
    .iter()
    .map(|variant| enum_display_arm(variant, &formatter, &mut bounds))
    .collect::<Result<Vec<_>>>()?;
  let imports = use_as_display(
    input
      .variants
      .iter()
      .any(|variant| variant.attrs.display.as_ref().is_some_and(|display| display.uses_display_view)),
  );
  let dereference = input.variants.is_empty().then(|| quote!(*));
  let body = quote! {
    #imports
    match #dereference self { #(#arms,)* }
  };
  Ok(Some(display_impl(&input.ident, input.generics, &bounds, &formatter, &body)))
}

/// Generate a variant's display arm and retain its required field bounds.
///
/// # Errors
/// Returns an invalid inferred bound or missing transparent field.
#[allow(
  clippy::single_call_fn,
  reason = "variant display emission owns literal, custom-formatter, and transparent argument binding"
)]
fn enum_display_arm(variant: &Variant<'_>, formatter: &Ident, bounds: &mut InferredBounds) -> Result<TokenStream> {
  let body = if let Some(display) = variant.attrs.display.as_ref() {
    display_bounds(&variant.fields, &display.implied_bounds, bounds)?;
    display.render(formatter)
  } else if let Some(fmt) = variant.attrs.fmt.as_ref() {
    let path = &fmt.path;
    let variables = variant.fields.iter().map(field_binding);
    quote!(#path(#(#variables,)* #formatter))
  } else {
    let field = variant
      .fields
      .first()
      .ok_or_else(|| syn::Error::new_spanned(variant.original, "missing display field"))?;
    if field.needs_inferred_bounds {
      bounds.insert(field.ty, Trait::Display.to_token_stream())?;
    }
    let binding = field_binding(field);
    quote!(::core::fmt::Display::fmt(#binding, #formatter))
  };
  let name = &variant.ident;
  let referenced = variant.attrs.display.as_ref().map(display_references);
  let pattern = fields_pat(&variant.fields, referenced.as_ref());
  Ok(quote!(Self::#name #pattern => #body))
}

/// Record the complete original type for each field used by a formatting trait.
///
/// # Errors
/// Returns an invalid field index or malformed generated bound.
fn display_bounds(fields: &[Field<'_>], implied: &BTreeSet<(usize, Trait)>, bounds: &mut InferredBounds) -> Result<()> {
  for &(index, bound) in implied {
    let field = fields
      .get(index)
      .ok_or_else(|| syn::Error::new(Span::call_site(), "format argument has no corresponding field"))?;
    if field.needs_inferred_bounds {
      bounds.insert(field.ty, bound.to_token_stream())?;
    }
  }
  Ok(())
}

/// Assemble a display implementation using one hygienic formatter binding.
fn display_impl(ident: &Ident, generics: &syn::Generics, bounds: &InferredBounds, formatter: &Ident, body: &TokenStream) -> TokenStream {
  let ty = call_site_ident(ident);
  let (parameters, arguments, _) = generics.split_for_impl();
  let clause = bounds.augment_where_clause(generics);
  quote! {
    #[automatically_derived]
    impl #parameters ::core::fmt::Display for #ty #arguments #clause {
      fn fmt(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result { #body }
    }
  }
}

/// Assemble a conversion while retaining the source type, attribute span, and backtrace capture.
fn from_impl(
  ident: &Ident,
  generics: &syn::Generics,
  variant: Option<&Ident>,
  field: &Field<'_>,
  backtrace: Option<&Field<'_>>,
  span: Span,
) -> TokenStream {
  let ty = call_site_ident(ident);
  let (parameters, arguments, clause) = generics.split_for_impl();
  let from = unoptional_type(field.ty);
  let source = Ident::new("source", span);
  let body = from_initializer(field, backtrace, &source);
  let constructor = variant.map_or_else(|| quote!(#ty), |name| quote!(#ty::#name));
  quote_spanned! {span=>
    #[automatically_derived]
    impl #parameters ::core::convert::From<#from> for #ty #arguments #clause {
      fn from(#source: #from) -> Self { #constructor #body }
    }
  }
}

/// Assemble the standard error implementation after completing source-bound inference.
///
/// # Errors
/// Returns invalid generated supertrait bound syntax.
fn error_impl(
  ident: &Ident,
  generics: &syn::Generics,
  bounds: &mut InferredBounds,
  source: Option<&TokenStream>,
  provide: Option<&TokenStream>,
) -> Result<TokenStream> {
  if generics.type_params().next().is_some() {
    let self_type = syn::parse2(quote!(Self))?;
    bounds.insert(&self_type, quote!(::core::fmt::Debug + ::core::fmt::Display))?;
  }
  let ty = call_site_ident(ident);
  let (parameters, arguments, _) = generics.split_for_impl();
  let clause = bounds.augment_where_clause(generics);
  Ok(quote! {
    #[automatically_derived]
    impl #parameters ::thiserror::#Private::Error for #ty #arguments #clause {
      #source #provide
    }
  })
}

/// Resolve a type name at the derive invocation without changing its diagnostic span.
pub(super) fn call_site_ident(ident: &Ident) -> Ident {
  let mut resolved = ident.clone();
  resolved.set_span(ident.span().resolved_at(Span::call_site()));
  resolved
}

/// Bind named and numbered fields through the same complete braced pattern.
fn fields_pat(fields: &[Field<'_>], referenced: Option<&BTreeSet<IdentUnraw>>) -> TokenStream {
  let bindings = fields.iter().map(|field| {
    let binding = field_binding(field);
    if referenced.is_some_and(|names| !names.contains(&IdentUnraw::new(binding.clone()))) {
      let member = &field.member;
      return quote!(#member: _);
    }
    match field.member {
      MemberUnraw::Named(ref ident) => {
        let mut member = ident.to_local();
        member.set_span(Span::call_site());
        quote!(#member: #binding)
      }
      MemberUnraw::Unnamed(ref index) => quote!(#index: #binding),
    }
  });
  quote!({ #(#bindings),* })
}

/// Retain native identifier references from explicit and generated formatting expressions.
fn display_references(display: &super::attr::Display<'_>) -> BTreeSet<IdentUnraw> {
  let mut references = BTreeSet::new();
  identifier_references(display.args.clone(), &mut references);
  for binding in &display.bindings {
    identifier_references(binding.1.clone(), &mut references);
  }
  references
}

/// Traverse native token groups without rendering or matching source fragments.
fn identifier_references(tokens: TokenStream, references: &mut BTreeSet<IdentUnraw>) {
  for token in tokens {
    match token {
      TokenTree::Ident(ident) => references.extend([IdentUnraw::new(ident)]),
      TokenTree::Group(group) => identifier_references(group.stream(), references),
      TokenTree::Punct(_) | TokenTree::Literal(_) => {}
    }
  }
}

/// Resolve a local binding for an original named or numbered field.
fn field_binding(field: &Field<'_>) -> Ident {
  match field.member {
    MemberUnraw::Named(ref ident) => ident.to_local(),
    MemberUnraw::Unnamed(ref index) => format_ident!("field_{}", index),
  }
}

/// Import the display-view contract only when shorthand actually uses it.
fn use_as_display(required: bool) -> Option<TokenStream> {
  required.then(|| quote!(use ::thiserror::#Private::AsDisplay as _;))
}

/// Forward a request to a concrete source, borrowing optional sources only when present.
fn source_provide_body(field: &Field<'_>, source: &TokenStream, span: Span, request: &TokenStream) -> TokenStream {
  if type_is_option(field.ty) {
    quote_spanned! {span=>
      if let ::core::option::Option::Some(optional_source) = &#source {
        optional_source.thiserror_provide(#request);
      }
    }
  } else {
    quote_spanned!(span=> #source.thiserror_provide(#request);)
  }
}

/// Register a bound backtrace, borrowing optional backtraces only when present.
#[allow(
  clippy::single_call_fn,
  reason = "enum provider arms share the optional-versus-required backtrace registration rule"
)]
fn backtrace_provide_body(field: &Field<'_>, request: &TokenStream) -> TokenStream {
  if type_is_option(field.ty) {
    quote! {
      if let ::core::option::Option::Some(backtrace) = backtrace {
        #request.provide_ref::<::thiserror::#Private::Backtrace>(backtrace);
      }
    }
  } else {
    quote!(#request.provide_ref::<::thiserror::#Private::Backtrace>(backtrace);)
  }
}

/// Construct the original source field and capture an independent backtrace when requested.
#[allow(
  clippy::single_call_fn,
  reason = "conversion emission owns optional-source wrapping and independent backtrace capture"
)]
fn from_initializer(field: &Field<'_>, backtrace: Option<&Field<'_>>, source: &Ident) -> TokenStream {
  let member = &field.member;
  let source_initializer = if type_is_option(field.ty) {
    quote!(::core::option::Option::Some(#source))
  } else {
    quote!(#source)
  };
  let backtrace_initializer = backtrace.map(|backtrace_field| {
    let trace_member = &backtrace_field.member;
    if type_is_option(backtrace_field.ty) {
      quote!(#trace_member: ::core::option::Option::Some(::thiserror::#Private::Backtrace::capture()),)
    } else {
      quote!(#trace_member: ::core::convert::From::from(::thiserror::#Private::Backtrace::capture()),)
    }
  });
  quote!({ #member: #source_initializer, #backtrace_initializer })
}

/// Whether a complete parsed type uses a single type argument on a final Option segment.
fn type_is_option(ty: &Type) -> bool {
  type_parameter_of_option(ty).is_some()
}

/// Retain a source's complete native type while unwrapping one recognized Option layer.
fn unoptional_type(ty: &Type) -> &Type {
  type_parameter_of_option(ty).unwrap_or(ty)
}

/// Extract the sole native type argument from a recognized Option type.
fn type_parameter_of_option(ty: &Type) -> Option<&Type> {
  let Type::Path(ref path) = *ty else {
    return None;
  };
  let segment = path.path.segments.last()?;
  if segment.ident != "Option" {
    return None;
  }
  let PathArguments::AngleBracketed(ref bracketed) = segment.arguments else {
    return None;
  };
  let mut arguments = bracketed.args.iter();
  let first = arguments.next()?;
  if arguments.next().is_some() {
    return None;
  }
  let GenericArgument::Type(ref contained) = *first else {
    return None;
  };
  Some(contained)
}
