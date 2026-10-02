use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use syn::Generics;
use syn::Ident;
use syn::PredicateType;
use syn::Result;
use syn::Token;
use syn::Type;
use syn::TypeParamBound;
use syn::TypePath;
use syn::WhereClause;
use syn::WherePredicate;
use syn::parse::Parser as _;
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::visit::visit_type_path;

/// Type parameter names declared by the enclosing error.
pub(super) struct ParamsInScope<'a> {
  /// Borrowed identifiers preserve the original declaration's scope.
  names: BTreeSet<&'a Ident>,
}

impl<'a> ParamsInScope<'a> {
  /// Collect the enclosing declaration's type parameters without interpreting lifetimes or consts.
  pub(super) fn new(generics: &'a Generics) -> Self {
    Self {
      names: generics.type_params().map(|param| &param.ident).collect(),
    }
  }

  /// Infer whole-field bounds only for generic types that do not refer to the enclosing error.
  ///
  /// A bound containing `Self` would require the implementation currently being generated,
  /// making otherwise valid recursive `Error` and `Display` implementations circular.
  pub(super) fn needs_inferred_bounds(&self, ty: &Type) -> bool {
    let mut parameters = TypeParameters {
      scope:     self,
      generic:   false,
      recursive: false,
    };
    parameters.visit_type(ty);
    parameters.generic && !parameters.recursive
  }
}

/// Discover generic parameters and recursive `Self` through the complete parsed type.
struct TypeParameters<'scope, 'generics> {
  /// Type parameter names belonging to the enclosing declaration.
  scope:     &'scope ParamsInScope<'generics>,
  /// Whether an enclosing type parameter occurs in the field.
  generic:   bool,
  /// Whether the field refers to the enclosing error through `Self`.
  recursive: bool,
}

impl<'ast> Visit<'ast> for TypeParameters<'_, '_> {
  fn visit_type_path(&mut self, ty: &'ast TypePath) {
    if ty.qself.is_none()
      && let Some(front) = ty.path.segments.first()
    {
      self.generic |= front.arguments.is_none() && self.scope.names.contains(&front.ident);
      self.recursive |= front.ident == "Self";
    }
    visit_type_path(self, ty);
  }
}

/// Complete inferred predicates in field order, deduplicated by native syntax equality.
pub(super) struct InferredBounds {
  /// Each predicate owns its field type and its ordered trait and lifetime bounds.
  predicates: Vec<PredicateType>,
}

impl InferredBounds {
  /// Start with no inferred field predicates.
  pub(super) const fn new() -> Self {
    Self {
      predicates: Vec::new()
    }
  }

  /// Merge generated bounds into the predicate for the complete original field type.
  ///
  /// # Errors
  /// Returns the native parser error if a generated bound is not valid bound syntax.
  pub(super) fn insert(&mut self, ty: &Type, bound: TokenStream) -> Result<()> {
    let parsed = Punctuated::<TypeParamBound, Token![+]>::parse_separated_nonempty.parse2(bound)?;
    let Some(predicate) = self.predicates.iter_mut().find(|predicate| predicate.bounded_ty == *ty) else {
      self.predicates.push(PredicateType {
        attrs:       Vec::new(),
        lifetimes:   None,
        bounded_ty:  ty.clone(),
        colon_token: <Token![:]>::default(),
        bounds:      parsed,
      });
      return Ok(());
    };
    for candidate in parsed {
      if predicate.bounds.iter().any(|existing| *existing == candidate) {
        continue;
      }
      predicate.bounds.push(candidate);
    }
    Ok(())
  }

  /// Append inferred predicates to the original user-authored where clause.
  pub(super) fn augment_where_clause(&self, generics: &Generics) -> WhereClause {
    let mut clause = generics.where_clause.clone().unwrap_or_else(|| WhereClause {
      where_token: <Token![where]>::default(),
      predicates:  Punctuated::new(),
    });
    clause
      .predicates
      .extend(self.predicates.iter().cloned().map(WherePredicate::Type));
    clause
  }
}

/// Native parsed declarations exercise generic discovery independently of generated source text.
#[cfg(test)]
mod tests {
  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use syn::Data;
  use syn::DeriveInput;
  use syn::Error;

  use super::ParamsInScope;

  /// Complete native parsing result for a batch of declaration fixtures.
  type ParsedDeclarations = Result<Vec<DeriveInput>, Error>;

  /// Check each complete native field type against the enclosing declaration's inference rule.
  fn fields_follow_inference(parsed: &ParsedDeclarations, expected: bool) -> bool {
    let Ok(declarations) = parsed.as_ref() else {
      return false;
    };
    declarations.iter().all(|declaration| {
      let Data::Struct(ref syntax) = declaration.data else {
        return false;
      };
      let scope = ParamsInScope::new(&declaration.generics);
      syntax
        .fields
        .iter()
        .all(|field| scope.needs_inferred_bounds(&field.ty) == expected)
    })
  }

  /// Whole-field inference must reject nested Self regardless of its structural position.
  #[test]
  fn recursive_types_do_not_infer_circular_bounds() -> Result<(), PredicateFailure<ParsedDeclarations>> {
    let declarations = [
      "struct Example<T>(Pair<Self, T>);",
      "struct Example<T>(Pair<T, (Self,)>);",
      "struct Example<T>(Pair<T, &'static Self>);",
      "struct Example<T>(Pair<T, <Self as Associated>::Output>);",
      "struct Example<T>(Pair<T, fn() -> Self>);",
    ]
    .into_iter()
    .map(syn::parse_str::<DeriveInput>)
    .collect::<ParsedDeclarations>();
    ensure_that(
      declarations,
      "nested Self never introduces a bound requiring the enclosing implementation",
      |observed| fields_follow_inference(observed, false),
    )
    .map(drop)
  }

  /// Ordinary generic types continue to receive bounds through all parsed type forms.
  #[test]
  fn nonrecursive_generics_keep_inferred_bounds() -> Result<(), PredicateFailure<ParsedDeclarations>> {
    let declarations = [
      "struct Example<T>(T);",
      "struct Example<T>(Box<T>);",
      "struct Example<T>((T, u8));",
      "struct Example<T>(&'static T);",
      "struct Example<T>(<T as Associated>::Output);",
    ]
    .into_iter()
    .map(syn::parse_str::<DeriveInput>)
    .collect::<ParsedDeclarations>();
    ensure_that(declarations, "normal generics retain their source and display bounds", |observed| {
      fields_follow_inference(observed, true)
    })
    .map(drop)
  }
}
