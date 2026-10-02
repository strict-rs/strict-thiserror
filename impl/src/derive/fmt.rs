use std::collections::BTreeSet;
use std::collections::HashMap;
use std::iter;
use std::num::ParseIntError;
use std::result::Result as NativeResult;

use proc_macro2::Delimiter;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::ToTokens as _;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use syn::Expr;
use syn::Ident;
use syn::Index;
use syn::LitStr;
use syn::Token;
use syn::ext::IdentExt as _;
use syn::parse::Error;
use syn::parse::ParseStream;
use syn::parse::Parser as _;
use syn::parse::Result;
use syn::parse::discouraged::Speculative as _;

use super::ast::ContainerKind;
use super::ast::Field;
use super::attr::Display;
use super::attr::Trait;
use super::unraw::IdentUnraw;
use super::unraw::MemberUnraw;
use crate::Private;

impl Display<'_> {
  /// Render the selected formatting operation with the generator's hygienic formatter binding.
  pub(super) fn render(&self, formatter: &Ident) -> TokenStream {
    let fmt = &self.fmt;
    let args = &self.args;

    // Currently `write!(f, "text")` produces less efficient code than
    // `f.write_str("text")`. We recognize the case when the format string
    // has no braces and no interpolated values, and generate simpler code.
    let write = if self.requires_fmt_machinery {
      quote! {
          ::core::write!(#formatter, #fmt #args)
      }
    } else {
      quote! {
          #formatter.write_str(#fmt)
      }
    };

    if self.bindings.is_empty() {
      write
    } else {
      let locals = self.bindings.iter().map(|binding| &binding.0);
      let values = self.bindings.iter().map(|binding| &binding.1);
      quote! {
          match (#(#values,)*) {
              (#(#locals,)*) => #write
          }
      }
    }
  }

  /// Resolve field shorthand while preserving opaque user expressions for the Rust compiler.
  ///
  /// # Errors
  /// Rejects ambiguous numbered arguments, recursive self-display, and malformed expression syntax.
  pub(super) fn expand_shorthand(&mut self, fields: &[Field<'_>], container: ContainerKind) -> Result<()> {
    let arguments = explicit_named_args.parse2(self.args.clone())?;
    let members = fields
      .iter()
      .enumerate()
      .map(|(index, field)| (&field.member, (index, field)))
      .collect::<HashMap<_, _>>();
    let extra_positional = fields.iter().all(|field| matches!(&field.member, MemberUnraw::Named(_)));
    let span = self.fmt.span();
    let message = self.fmt.value();
    let mut read = message.as_str();
    let mut out = String::new();
    let mut bonus = false;
    let mut implied = BTreeSet::new();
    let mut bindings = Vec::new();
    let mut names = BTreeSet::new();
    let mut requires_format = self.requires_fmt_machinery || message.contains('}');
    while let Some((prefix, tail)) = read.split_once('{') {
      requires_format = true;
      out.push_str(prefix);
      out.push('{');
      read = tail;
      if let Some(after_escape) = read.strip_prefix('{') {
        out.push('{');
        read = after_escape;
        continue;
      }
      let Some(next) = read.chars().next() else {
        return Ok(());
      };
      if next.is_ascii_digit()
        && !extra_positional
        && let Some(unnamed) = arguments.first_unnamed.as_ref()
      {
        return Err(Error::new_spanned(
          unnamed,
          format!("ambiguous reference to positional arguments by number in a {container}; change this to a named argument"),
        ));
      }
      let Ok(recognized) = shorthand_member(next, &mut read, &mut out, span, &arguments.named) else {
        return Ok(());
      };
      let Some(member) = recognized else {
        continue;
      };
      let Some((specifier, _tail)) = read.split_once('}') else {
        return Ok(());
      };
      let (bound, display_view) = format_bound(specifier);
      if member == *"self" && bound == Trait::Display {
        return Err(Error::new(
          span,
          "displaying self recursively is not supported; use a field or a Debug format",
        ));
      }
      bonus |= display_view;
      let Some(&(index, field)) = members.get(&member) else {
        out.push_str(&member.to_string());
        continue;
      };
      implied.extend([(index, bound)]);
      let (name, expression) = field_format_argument(&member, field, bound, display_view, span, &arguments.named, &mut names);
      out.push_str(&name.to_string());
      if let Some(bound_expression) = expression {
        bindings.push((name.to_local(), bound_expression));
      }
    }
    out.push_str(read);
    self.fmt = LitStr::new(&out, span);
    self.requires_fmt_machinery = requires_format;
    self.uses_display_view = bonus;
    self.implied_bounds = implied;
    self.bindings = bindings;
    Ok(())
  }
}

/// Resolve a named or numbered shorthand member without consuming its format specifier.
///
/// # Errors
/// Returns the native integer parser failure for an unrepresentable tuple field number.
#[allow(
  clippy::single_call_fn,
  reason = "member recognition preserves the distinct named, numbered, and opaque shorthand outcomes"
)]
fn shorthand_member(
  next: char,
  read: &mut &str,
  out: &mut String,
  span: Span,
  named: &BTreeSet<IdentUnraw>,
) -> NativeResult<Option<MemberUnraw>, ParseIntError> {
  match next {
    '0'..='9' => take_int(read).parse::<u32>().map(|index| {
      Some(MemberUnraw::Unnamed(Index {
        index,
        span,
      }))
    }),
    'a'..='z' | 'A'..='Z' | '_' => Ok(named_member_shorthand(read, out, span, named)),
    _ => Ok(None),
  }
}

/// Resolve the standard formatting trait and whether the runtime must select a display view.
#[allow(
  clippy::single_call_fn,
  reason = "format specifier interpretation determines both the field bound and its runtime display adaptation"
)]
fn format_bound(specifier: &str) -> (Trait, bool) {
  match specifier.chars().next_back() {
    Some('?') => (Trait::Debug, false),
    Some('o') => (Trait::Octal, false),
    Some('x') => (Trait::LowerHex, false),
    Some('X') => (Trait::UpperHex, false),
    Some('p') => (Trait::Pointer, false),
    Some('b') => (Trait::Binary, false),
    Some('e') => (Trait::LowerExp, false),
    Some('E') => (Trait::UpperExp, false),
    Some(_) => (Trait::Display, false),
    None => (Trait::Display, true),
  }
}

/// Build a hygienic formatting argument once, retaining its complete original field binding.
#[allow(
  clippy::single_call_fn,
  reason = "field argument construction owns name collisions, deduplication, and runtime pointer/display views"
)]
fn field_format_argument(
  member: &MemberUnraw,
  field: &Field<'_>,
  bound: Trait,
  display_view: bool,
  span: Span,
  user_names: &BTreeSet<IdentUnraw>,
  generated_names: &mut BTreeSet<IdentUnraw>,
) -> (IdentUnraw, Option<TokenStream>) {
  let prefix = if display_view {
    "display_argument"
  } else if bound == Trait::Pointer {
    "pointer_argument"
  } else {
    "field_argument"
  };
  let mut name = IdentUnraw::new(match *member {
    MemberUnraw::Unnamed(ref index) => format_ident!("{prefix}{}", index.index),
    MemberUnraw::Named(ref ident) => format_ident!("{prefix}_{ident}"),
  });
  while user_names.contains(&name) {
    name = IdentUnraw::new(format_ident!("generated_{name}"));
  }
  name.set_span(span);
  if !generated_names.insert(name.clone()) {
    return (name, None);
  }
  let mut binding = match *member {
    MemberUnraw::Unnamed(ref index) => format_ident!("field_{}", index.index),
    MemberUnraw::Named(ref ident) => ident.to_local(),
  };
  binding.set_span(span.resolved_at(field.member.span()));
  let expression = if display_view {
    quote_spanned!(span=> #binding.as_display())
  } else if bound == Trait::Pointer {
    quote!(::thiserror::#Private::Var(#binding))
  } else {
    binding.into_token_stream()
  };
  (name, Some(expression))
}

/// Resolve an identifier interpolation while preserving raw, invalid, and explicitly named
/// arguments.
#[allow(
  clippy::single_call_fn,
  reason = "named shorthand distinguishes field references from raw and explicitly supplied formatting arguments"
)]
fn named_member_shorthand(read: &mut &str, out: &mut String, span: Span, names: &BTreeSet<IdentUnraw>) -> Option<MemberUnraw> {
  if read.starts_with("r#") {
    return None;
  }
  let repr = take_ident(read)?;
  if repr == "_" {
    out.push_str(repr);
    return None;
  }
  let ident = IdentUnraw::new(Ident::new(repr, span));
  if names.contains(&ident) {
    out.push_str(repr);
    None
  } else {
    Some(MemberUnraw::Named(ident))
  }
}

/// Original explicit argument names and the first positional expression, if present.
struct FmtArguments {
  /// Explicit names take precedence over shorthand field names.
  named:         BTreeSet<IdentUnraw>,
  /// Complete first unnamed expression retained for ambiguity diagnostics.
  first_unnamed: Option<TokenStream>,
}

/// Discover explicit argument names without replacing Rust's own expression diagnostics.
///
/// # Errors
/// Returns a native token parsing failure when the original arguments cannot be consumed.
#[allow(
  clippy::single_call_fn,
  reason = "argument discovery preserves full-expression parsing and opaque-token fallback as one contract"
)]
fn explicit_named_args(input: ParseStream<'_>) -> Result<FmtArguments> {
  let ahead = input.fork();
  if let Ok(arguments) = try_explicit_named_args(&ahead) {
    input.advance_to(&ahead);
    return Ok(arguments);
  }
  let fallback_cursor = input.fork();
  if let Ok(arguments) = fallback_explicit_named_args(&fallback_cursor) {
    input.advance_to(&fallback_cursor);
    return Ok(arguments);
  }
  input.parse::<TokenStream>().map(drop)?;
  Ok(FmtArguments {
    named:         BTreeSet::new(),
    first_unnamed: None,
  })
}

/// Consume complete expressions through the workspace's required full Syn parser.
///
/// # Errors
/// Returns the original argument punctuation or expression parsing failure.
#[allow(
  clippy::single_call_fn,
  reason = "complete expression discovery retains positional spans and explicit names before opaque fallback"
)]
fn try_explicit_named_args(input: ParseStream<'_>) -> Result<FmtArguments> {
  let mut arguments = FmtArguments {
    named:         BTreeSet::new(),
    first_unnamed: None,
  };
  while !input.is_empty() {
    input.parse::<Token![,]>().map(drop)?;
    if input.is_empty() {
      break;
    }
    let unnamed = if input.peek(Ident::peek_any) && input.peek2(Token![=]) && !input.peek2(Token![==]) {
      let ident = input.parse::<IdentUnraw>()?;
      input.parse::<Token![=]>().map(drop)?;
      arguments.named.extend([ident]);
      None
    } else {
      Some(input.fork())
    };
    input.parse::<Expr>().map(drop)?;
    if let Some(begin) = unnamed
      && arguments.first_unnamed.is_none()
    {
      arguments.first_unnamed = Some(between(&begin, input));
    }
  }
  Ok(arguments)
}

/// Retain explicit names when syntax must remain opaque for the Rust compiler.
///
/// # Errors
/// Returns a malformed token or named-argument punctuation error.
#[allow(
  clippy::single_call_fn,
  reason = "opaque fallback preserves caller expressions that the full parser cannot yet interpret"
)]
fn fallback_explicit_named_args(input: ParseStream<'_>) -> Result<FmtArguments> {
  let mut arguments = FmtArguments {
    named:         BTreeSet::new(),
    first_unnamed: None,
  };
  while !input.is_empty() {
    if input.peek(Token![,]) && input.peek2(Ident::peek_any) && input.peek3(Token![=]) && !input.peek3(Token![==]) {
      input.parse::<Token![,]>().map(drop)?;
      let ident = input.parse::<IdentUnraw>()?;
      input.parse::<Token![=]>().map(drop)?;
      arguments.named.extend([ident]);
    } else {
      input.parse::<TokenTree>().map(drop)?;
    }
  }
  Ok(arguments)
}

/// Consume only the leading decimal bytes without indexing or integer arithmetic.
#[allow(
  clippy::single_call_fn,
  reason = "numbered shorthand must leave its complete format specifier unconsumed"
)]
fn take_int<'a>(read: &mut &'a str) -> &'a str {
  let length = read.find(|character: char| !character.is_ascii_digit()).unwrap_or(read.len());
  let Some((integer, rest)) = read.split_at_checked(length) else {
    return "";
  };
  *read = rest;
  integer
}

/// Consume only a leading shorthand identifier without changing the following specifier.
#[allow(
  clippy::single_call_fn,
  reason = "named shorthand preserves the exact boundary before its format specifier"
)]
fn take_ident<'a>(read: &mut &'a str) -> Option<&'a str> {
  let length = read
    .find(|character: char| !character.is_ascii_alphanumeric() && character != '_')
    .unwrap_or(read.len());
  let (ident, rest) = read.split_at_checked(length)?;
  *read = rest;
  Some(ident)
}

/// Retain the complete token range between two parsed expression cursors.
#[allow(
  clippy::single_call_fn,
  reason = "positional ambiguity diagnostics retain the native first expression rather than a rendered reconstruction"
)]
fn between<'a>(begin: ParseStream<'a>, end: ParseStream<'a>) -> TokenStream {
  let boundary = end.cursor();
  let mut cursor = begin.cursor();
  let mut tokens = TokenStream::new();
  while cursor < boundary {
    let Some((token, next)) = cursor.token_tree() else {
      break;
    };
    if boundary < next {
      if let Some((inside, _span, _after)) = cursor.group(Delimiter::None) {
        cursor = inside;
        continue;
      }
      if tokens.is_empty() {
        tokens.extend(iter::once(token));
      }
      break;
    }
    tokens.extend(iter::once(token));
    cursor = next;
  }
  tokens
}
