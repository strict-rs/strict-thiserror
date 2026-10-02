use std::collections::BTreeSet as Set;

use proc_macro2::Delimiter;
use proc_macro2::Group;
use proc_macro2::Literal;
use proc_macro2::Punct;
use proc_macro2::Spacing;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens;
use quote::format_ident;
use quote::quote;
use syn::Attribute;
use syn::Error;
use syn::ExprPath;
use syn::Ident;
use syn::Index;
use syn::LitFloat;
use syn::LitInt;
use syn::LitStr;
use syn::Meta;
use syn::Result;
use syn::Token;
use syn::braced;
use syn::bracketed;
use syn::parenthesized;
use syn::parse::End;
use syn::parse::ParseStream;
use syn::parse::discouraged::Speculative as _;
use syn::spanned::Spanned as _;
use syn::token;

/// Derive attributes belonging to one declaration, variant, or field.
pub(super) struct Attrs<'a> {
  /// Literal display message and formatting arguments.
  pub(super) display:     Option<Display<'a>>,
  /// Explicit standard error source.
  pub(super) source:      Option<Source<'a>>,
  /// Explicit backtrace provider or capture field.
  pub(super) backtrace:   Option<&'a Attribute>,
  /// Conversion source for an automatically derived `From` implementation.
  pub(super) from:        Option<From<'a>>,
  /// Delegate display and source selection to the sole field.
  pub(super) transparent: Option<Transparent<'a>>,
  /// Custom enum-variant formatter.
  pub(super) fmt:         Option<Fmt<'a>>,
}

/// Format message after resolving field shorthand and explicit arguments.
#[derive(Clone)]
pub(super) struct Display<'a> {
  /// Original attribute retained for validation diagnostics.
  pub(super) original:               &'a Attribute,
  /// Resolved message passed to the selected formatting operation.
  pub(super) fmt:                    LitStr,
  /// Explicit arguments with field shorthand rewritten to local bindings.
  pub(super) args:                   TokenStream,
  /// Whether formatting arguments or escaped braces require the formatting machinery.
  pub(super) requires_fmt_machinery: bool,
  /// Whether shorthand formatting requires the runtime's display-view contract.
  pub(super) uses_display_view:      bool,
  /// Field indices and their required formatting traits.
  pub(super) implied_bounds:         Set<(usize, Trait)>,
  /// Generated argument names and the expressions they borrow.
  pub(super) bindings:               Vec<(Ident, TokenStream)>,
}

/// Explicit source attribute and its diagnostic span.
#[derive(Copy, Clone)]
pub(super) struct Source<'a> {
  /// Complete original attribute.
  pub(super) original: &'a Attribute,
  /// Span covering the attribute syntax when the parser can join it.
  pub(super) span:     Span,
}

/// Conversion attribute and its diagnostic span.
#[derive(Copy, Clone)]
pub(super) struct From<'a> {
  /// Complete original attribute.
  pub(super) original: &'a Attribute,
  /// Span covering the conversion attribute.
  pub(super) span:     Span,
}

/// Transparent delegation attribute and its diagnostic span.
#[derive(Copy, Clone)]
pub(super) struct Transparent<'a> {
  /// Complete original attribute.
  pub(super) original: &'a Attribute,
  /// Span covering the transparent keyword.
  pub(super) span:     Span,
}

/// Path to the user-selected enum formatter.
#[derive(Clone)]
pub(super) struct Fmt<'a> {
  /// Complete original attribute.
  pub(super) original: &'a Attribute,
  /// Formatter expression path retained with its original hygiene.
  pub(super) path:     ExprPath,
}

/// Standard formatting trait required by one field interpolation.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub(super) enum Trait {
  /// Debug representation.
  Debug,
  /// User-facing display representation.
  Display,
  /// Octal integer representation.
  Octal,
  /// Lowercase hexadecimal representation.
  LowerHex,
  /// Uppercase hexadecimal representation.
  UpperHex,
  /// Pointer identity.
  Pointer,
  /// Binary integer representation.
  Binary,
  /// Lowercase exponential representation.
  LowerExp,
  /// Uppercase exponential representation.
  UpperExp,
}

/// Parse the derive's attributes without interpreting attributes owned by other derives.
///
/// # Errors
/// Rejects malformed or repeated attributes owned by this derive.
pub(super) fn get(input: &[Attribute]) -> Result<Attrs<'_>> {
  let mut attrs = Attrs {
    display:     None,
    source:      None,
    backtrace:   None,
    from:        None,
    transparent: None,
    fmt:         None,
  };

  for attr in input {
    if attr.path().is_ident("error") {
      parse_error_attribute(&mut attrs, attr)?;
    } else if attr.path().is_ident("source") {
      attr.meta.require_path_only().map(drop)?;
      if attrs.source.is_some() {
        return Err(Error::new_spanned(attr, "duplicate #[source] attribute"));
      }
      let span = (attr.pound_token.span)
        .join(attr.bracket_token.span.join())
        .unwrap_or_else(|| attr.path().span());
      attrs.source = Some(Source {
        original: attr,
        span,
      });
    } else if attr.path().is_ident("backtrace") {
      attr.meta.require_path_only().map(drop)?;
      if attrs.backtrace.is_some() {
        return Err(Error::new_spanned(attr, "duplicate #[backtrace] attribute"));
      }
      attrs.backtrace = Some(attr);
    } else if attr.path().is_ident("from") {
      match attr.meta {
        Meta::Path(_) => {}
        Meta::List(_) | Meta::NameValue(_) => {
          // Assume this is meant for derive_more crate or something.
          continue;
        }
      }
      if attrs.from.is_some() {
        return Err(Error::new_spanned(attr, "duplicate #[from] attribute"));
      }
      let span = (attr.pound_token.span)
        .join(attr.bracket_token.span.join())
        .unwrap_or_else(|| attr.path().span());
      attrs.from = Some(From {
        original: attr,
        span,
      });
    } else {
      // Other derive owners retain their original attributes.
    }
  }

  Ok(attrs)
}

/// Parse one message, transparent delegation, or custom formatter attribute.
///
/// # Errors
/// Rejects duplicate forms or syntax that does not match a supported error attribute.
#[allow(
  clippy::single_call_fn,
  reason = "message, transparent, and custom-format attributes share one parsing and duplicate-validation boundary"
)]
fn parse_error_attribute<'a>(attrs: &mut Attrs<'a>, attr: &'a Attribute) -> Result<()> {
  /// Keywords reserved by the error attribute grammar.
  mod kw {
    syn::custom_keyword!(transparent);
    syn::custom_keyword!(fmt);
  }

  attr.parse_args_with(|input: ParseStream<'_>| {
    let lookahead = input.lookahead1();
    let fmt = if lookahead.peek(LitStr) {
      input.parse::<LitStr>()?
    } else if lookahead.peek(kw::transparent) {
      let kw: kw::transparent = input.parse()?;
      if attrs.transparent.is_some() {
        return Err(Error::new_spanned(attr, "duplicate #[error(transparent)] attribute"));
      }
      attrs.transparent = Some(Transparent {
        original: attr,
        span:     kw.span,
      });
      return Ok(());
    } else if lookahead.peek(kw::fmt) {
      input.parse::<kw::fmt>().map(drop)?;
      input.parse::<Token![=]>().map(drop)?;
      let path: ExprPath = input.parse()?;
      if attrs.fmt.is_some() {
        return Err(Error::new_spanned(attr, "duplicate #[error(fmt = ...)] attribute"));
      }
      attrs.fmt = Some(Fmt {
        original: attr,
        path,
      });
      return Ok(());
    } else {
      return Err(lookahead.error());
    };

    let args = if input.is_empty() || input.peek(Token![,]) && input.peek2(End) {
      input.parse::<Option<Token![,]>>().map(drop)?;
      TokenStream::new()
    } else {
      parse_token_expr(input, false)?
    };

    let requires_fmt_machinery = !args.is_empty();

    let display = Display {
      original: attr,
      fmt,
      args,
      requires_fmt_machinery,
      uses_display_view: false,
      implied_bounds: Set::new(),
      bindings: Vec::new(),
    };
    if attrs.display.is_some() {
      return Err(Error::new_spanned(attr, "only one #[error(...)] attribute is allowed"));
    }
    attrs.display = Some(display);
    Ok(())
  })
}

/// Rewrite leading field shorthand while retaining the rest of each expression's tokens.
///
/// # Errors
/// Returns the native parsing failure for an invalid token or delimited expression.
fn parse_token_expr(input: ParseStream<'_>, mut begin_expr: bool) -> Result<TokenStream> {
  let mut tokens = Vec::new();
  while !input.is_empty() {
    if input.peek(token::Group) {
      let group: TokenTree = input.parse()?;
      tokens.push(group);
      begin_expr = false;
      continue;
    }

    if begin_expr && input.peek(Token![.]) && consume_member_access(input, &mut tokens)? {
      begin_expr = false;
      continue;
    }

    begin_expr = input.peek(Token![break])
      || input.peek(Token![continue])
      || input.peek(Token![if])
      || input.peek(Token![in])
      || input.peek(Token![match])
      || input.peek(Token![mut])
      || input.peek(Token![return])
      || input.peek(Token![while])
      || input.peek(Token![+])
      || input.peek(Token![&])
      || input.peek(Token![!])
      || input.peek(Token![^])
      || input.peek(Token![,])
      || input.peek(Token![/])
      || input.peek(Token![=])
      || input.peek(Token![>])
      || input.peek(Token![<])
      || input.peek(Token![|])
      || input.peek(Token![%])
      || input.peek(Token![;])
      || input.peek(Token![*])
      || input.peek(Token![-]);

    let token: TokenTree = if input.peek(token::Paren) {
      let content;
      let delimiter = parenthesized!(content in input);
      let nested = parse_token_expr(&content, true)?;
      let mut group = Group::new(Delimiter::Parenthesis, nested);
      group.set_span(delimiter.span.join());
      TokenTree::Group(group)
    } else if input.peek(token::Brace) {
      let content;
      let delimiter = braced!(content in input);
      let nested = parse_token_expr(&content, true)?;
      let mut group = Group::new(Delimiter::Brace, nested);
      group.set_span(delimiter.span.join());
      TokenTree::Group(group)
    } else if input.peek(token::Bracket) {
      let content;
      let delimiter = bracketed!(content in input);
      let nested = parse_token_expr(&content, true)?;
      let mut group = Group::new(Delimiter::Bracket, nested);
      group.set_span(delimiter.span.join());
      TokenTree::Group(group)
    } else {
      input.parse()?
    };
    tokens.push(token);
  }
  Ok(TokenStream::from_iter(tokens))
}

/// Consume one member access after a leading `.` in expression position:
/// `.ident` keeps the identifier as a local binding reference, `.0` becomes
/// the `_0` binding, and a float literal such as `.0.0` is re-tokenized into
/// `_0 . 0`. Returns `Ok(false)` without consuming anything when the tokens
/// after the dot are not a member access this shorthand recognizes.
#[allow(
  clippy::single_call_fn,
  reason = "member access after a dot is one self-contained re-tokenization decision (identifier, integer index, or float index pair); \
            naming it keeps the token-expression scan loop at guard-clause depth"
)]
fn consume_member_access(input: ParseStream<'_>, tokens: &mut Vec<TokenTree>) -> Result<bool> {
  if input.peek2(Ident) {
    input.parse::<Token![.]>().map(drop)?;
    return Ok(true);
  }

  if input.peek2(LitInt) {
    input.parse::<Token![.]>().map(drop)?;
    let int: Index = input.parse()?;
    let ident = format_ident!("field_{}", int.index, span = int.span);
    tokens.push(TokenTree::Ident(ident));
    return Ok(true);
  }

  if input.peek2(LitFloat) {
    let ahead = input.fork();
    ahead.parse::<Token![.]>().map(drop)?;
    let float: LitFloat = ahead.parse()?;
    let repr = float.to_string();
    let mut indices = repr.split('.').map(syn::parse_str::<Index>);
    if let (Some(Ok(first)), Some(Ok(second)), None) = (indices.next(), indices.next(), indices.next()) {
      input.advance_to(&ahead);
      let ident = format_ident!("field_{}", first, span = float.span());
      tokens.push(TokenTree::Ident(ident));
      let mut punct = Punct::new('.', Spacing::Alone);
      punct.set_span(float.span());
      tokens.push(TokenTree::Punct(punct));
      let mut literal = Literal::u32_unsuffixed(second.index);
      literal.set_span(float.span());
      tokens.push(TokenTree::Literal(literal));
      return Ok(true);
    }
  }

  Ok(false)
}

impl ToTokens for Trait {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let trait_name = match *self {
      Self::Debug => "Debug",
      Self::Display => "Display",
      Self::Octal => "Octal",
      Self::LowerHex => "LowerHex",
      Self::UpperHex => "UpperHex",
      Self::Pointer => "Pointer",
      Self::Binary => "Binary",
      Self::LowerExp => "LowerExp",
      Self::UpperExp => "UpperExp",
    };
    let ident = Ident::new(trait_name, Span::call_site());
    tokens.extend(quote!(::core::fmt::#ident));
  }
}
