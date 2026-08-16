use std::collections::BTreeMap as Map;
use std::collections::BTreeSet as Set;
use std::collections::btree_map::Entry;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::GenericArgument;
use syn::Generics;
use syn::Ident;
use syn::PathArguments;
use syn::Token;
use syn::Type;
use syn::WhereClause;
use syn::parse_quote;
use syn::punctuated::Punctuated;

pub struct ParamsInScope<'a> {
  names: Set<&'a Ident>,
}

impl<'a> ParamsInScope<'a> {
  pub fn new(generics: &'a Generics) -> Self {
    ParamsInScope {
      names: generics.type_params().map(|param| &param.ident).collect(),
    }
  }

  pub fn intersects(&self, ty: &Type) -> bool {
    let mut found = false;
    crawl(self, ty, &mut found);
    found
  }
}

fn crawl(in_scope: &ParamsInScope, ty: &Type, found: &mut bool) {
  let Type::Path(ty) = ty else {
    return;
  };
  if let Some(qself) = &ty.qself {
    crawl(in_scope, &qself.ty, found);
  } else {
    let front = ty.path.segments.first().unwrap();
    if front.arguments.is_none() && in_scope.names.contains(&front.ident) {
      *found = true;
    }
  }
  for segment in &ty.path.segments {
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
      continue;
    };
    for arg in &arguments.args {
      if let GenericArgument::Type(ty) = arg {
        crawl(in_scope, ty, found);
      }
    }
  }
}

/// Bounds recorded against one type: the rendered forms already seen (for
/// deduplication) alongside the token bounds in insertion order.
type TypeBounds = (Set<String>, Punctuated<TokenStream, Token![+]>);

pub struct InferredBounds {
  bounds: Map<String, TypeBounds>,
  order:  Vec<TokenStream>,
}

impl InferredBounds {
  pub fn new() -> Self {
    InferredBounds {
      bounds: Map::new(),
      order:  Vec::new(),
    }
  }

  pub fn insert(&mut self, ty: impl ToTokens, bound: impl ToTokens) {
    let ty = ty.to_token_stream();
    let bound = bound.to_token_stream();
    let entry = self.bounds.entry(ty.to_string());
    if let Entry::Vacant(_) = entry {
      self.order.push(ty);
    }
    let (set, tokens) = entry.or_default();
    if set.insert(bound.to_string()) {
      tokens.push(bound);
    }
  }

  pub fn augment_where_clause(&self, generics: &Generics) -> WhereClause {
    let mut generics = generics.clone();
    let where_clause = generics.make_where_clause();
    for ty in &self.order {
      let (_set, bounds) = &self.bounds[&ty.to_string()];
      where_clause.predicates.push(parse_quote!(#ty: #bounds));
    }
    generics.where_clause.unwrap()
  }
}
