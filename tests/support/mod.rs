use std::error::Error;
use std::fmt::Display;

use strict_test_support::ComparisonFailure;
use strict_test_support::PredicateFailure;
use strict_test_support::ensure_eq;
use strict_test_support::ensure_that;

/// Failures from inspecting a concrete error's display and source chain.
#[derive(Debug, thiserror::Error)]
pub enum SourceFailure<Subject> {
  /// The error's display differed from the expected message.
  #[error(transparent)]
  Display(#[from] ComparisonFailure<String, String>),
  /// The original error did not expose the expected source type or absence.
  #[error(transparent)]
  Source(#[from] PredicateFailure<Subject>),
  /// The source display differed, retaining the owning error and both messages.
  #[error("{source}")]
  SourceDisplay {
    /// Concrete error that owns the inspected source chain.
    error:  Subject,
    /// Original source presence and rendering compared with the expectation.
    source: ComparisonFailure<Option<String>, Option<String>>,
  },
}

/// Compare a display implementation with its expected message.
///
/// # Errors
///
/// Returns both owned messages when the rendering differs from the expectation.
pub fn ensure_display<T: Display + ?Sized>(
  actual: &T,
  expected: &str,
  context: &'static str,
) -> Result<(), ComparisonFailure<String, String>> {
  let rendered = actual.to_string();
  let expected = expected.to_owned();
  ensure_eq(rendered, expected, context).map(drop)
}

/// Check a source's concrete type and display while retaining its owner on failure.
///
/// # Errors
///
/// Returns the owning error when its source is absent, has the wrong type, or renders a different
/// message.
pub fn ensure_source<Source, Subject>(
  error: Subject,
  expected: &str,
  type_context: &'static str,
  display_context: &'static str,
) -> Result<(), SourceFailure<Subject>>
where
  Source: Error + 'static,
  Subject: Error,
{
  let checked = ensure_that(error, type_context, |error| {
    error.source().and_then(|source| source.downcast_ref::<Source>()).is_some()
  })?;
  let rendered = checked.source().map(ToString::to_string);
  ensure_eq(rendered, Some(expected.to_owned()), display_context)
    .map(drop)
    .map_err(|source| SourceFailure::SourceDisplay {
      error: checked,
      source,
    })
}
