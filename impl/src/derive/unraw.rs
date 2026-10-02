use std::cmp::Ordering;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FormatResult;
use std::hash::Hash;
use std::hash::Hasher;

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::Index;
use syn::ext::IdentExt as _;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse::Result;

/// Identifier whose field comparisons and rendering ignore raw-identifier spelling.
#[derive(Clone)]
#[repr(transparent)]
pub(super) struct IdentUnraw(Ident);

impl IdentUnraw {
  /// Retain the original identifier with its native hygiene and span.
  pub(super) const fn new(ident: Ident) -> Self {
    Self(ident)
  }

  /// Produce a legal binding name without changing the identifier's meaning.
  pub(super) fn to_local(&self) -> Ident {
    let unraw = self.0.unraw();
    let repr = unraw.to_string();
    if syn::parse_str::<Ident>(&repr).is_err() {
      if let "_" | "super" | "self" | "Self" | "crate" = repr.as_str() {
        // Some identifiers are never allowed to appear as raw, like r#self and r#_.
      } else {
        return Ident::new_raw(&repr, Span::call_site());
      }
    }
    unraw
  }

  /// Resolve a generated binding at the corresponding original field span.
  pub(super) fn set_span(&mut self, span: Span) {
    self.0.set_span(span);
  }
}

impl Display for IdentUnraw {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
    Display::fmt(&self.0.unraw(), formatter)
  }
}

impl Eq for IdentUnraw {}

impl PartialEq for IdentUnraw {
  fn eq(&self, other: &Self) -> bool {
    PartialEq::eq(&self.0.unraw(), &other.0.unraw())
  }
}

impl PartialEq<str> for IdentUnraw {
  fn eq(&self, other: &str) -> bool {
    self.0 == other
  }
}

impl Ord for IdentUnraw {
  fn cmp(&self, other: &Self) -> Ordering {
    Ord::cmp(&self.0.unraw(), &other.0.unraw())
  }
}

impl PartialOrd for IdentUnraw {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(Self::cmp(self, other))
  }
}

impl Parse for IdentUnraw {
  fn parse(input: ParseStream<'_>) -> Result<Self> {
    input.call(Ident::parse_any).map(Self::new)
  }
}

impl ToTokens for IdentUnraw {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.0.unraw().to_tokens(tokens);
  }
}

/// Named or numbered field member retaining its original syntax.
#[derive(Clone)]
pub(super) enum MemberUnraw {
  /// Named field with raw-identifier normalization.
  Named(IdentUnraw),
  /// Numbered tuple field with its original span.
  Unnamed(Index),
}

impl MemberUnraw {
  /// Return the original field's diagnostic span.
  pub(super) fn span(&self) -> Span {
    match *self {
      Self::Named(ref ident) => ident.0.span(),
      Self::Unnamed(ref index) => index.span,
    }
  }
}

impl Display for MemberUnraw {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
    match *self {
      Self::Named(ref this) => Display::fmt(this, formatter),
      Self::Unnamed(ref this) => Display::fmt(&this.index, formatter),
    }
  }
}

impl Eq for MemberUnraw {}

impl PartialEq for MemberUnraw {
  fn eq(&self, other: &Self) -> bool {
    match *self {
      Self::Named(ref left) => match *other {
        Self::Named(ref right) => left == right,
        Self::Unnamed(_) => false,
      },
      Self::Unnamed(ref left) => match *other {
        Self::Unnamed(ref right) => left == right,
        Self::Named(_) => false,
      },
    }
  }
}

impl PartialEq<str> for MemberUnraw {
  fn eq(&self, other: &str) -> bool {
    match *self {
      Self::Named(ref this) => this == other,
      Self::Unnamed(_) => false,
    }
  }
}

impl Hash for MemberUnraw {
  fn hash<H: Hasher>(&self, hasher: &mut H) {
    match *self {
      Self::Named(ref ident) => ident.0.unraw().hash(hasher),
      Self::Unnamed(ref index) => index.hash(hasher),
    }
  }
}

impl ToTokens for MemberUnraw {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match *self {
      Self::Named(ref ident) => ident.to_local().to_tokens(tokens),
      Self::Unnamed(ref index) => index.to_tokens(tokens),
    }
  }
}
