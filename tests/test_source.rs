//! Native source selection preserves the concrete error and its original kind.

/// Source selection is observable through the standard error contract.
#[cfg(test)]
mod tests {
  use std::error::Error as StdError;
  use std::io::Error as IoError;
  use std::io::ErrorKind;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// A conventional field name selects the concrete source.
  #[derive(Debug, Error)]
  #[error("implicit source")]
  struct ImplicitSource {
    /// Original native I/O error.
    source: IoError,
  }

  /// An explicit attribute takes precedence over a non-error field named source.
  #[derive(Debug, Error)]
  #[error("explicit source")]
  struct ExplicitSource {
    /// Display-only data sharing the conventional source name.
    source: String,
    /// Original native I/O error explicitly selected as the source.
    #[source]
    io:     IoError,
  }

  /// Dynamic error interoperability retains the boxed native I/O source.
  #[derive(Debug, Error)]
  #[error("boxed source")]
  struct BoxedSource {
    /// Standard error object accepted by the source contract.
    #[source]
    source: Box<dyn StdError + Send + 'static>,
  }

  /// Macro-provided fields retain their call-site source identity.
  macro_rules! error_from_macro {
    ($variant:ident) => {
      /// Error declared through an outer macro.
      #[derive(Debug, Error)]
      enum MacroSource {
        /// Concrete I/O conversion declared by the caller.
        #[error("Something")]
        $variant(#[from] IoError),
      }
    };
  }

  error_from_macro!(Variant);

  /// Conventionally named source fields expose the original native kind.
  #[test]
  fn test_implicit_source() -> Result<(), PredicateFailure<ImplicitSource>> {
    let error = ImplicitSource {
      source: IoError::from(ErrorKind::NotFound),
    };
    ensure_that(error, "implicit source retains native I/O identity and kind", |subject| {
      subject.to_string() == "implicit source"
        && subject
          .source()
          .and_then(|cause| cause.downcast_ref::<IoError>())
          .is_some_and(|cause| cause.kind() == ErrorKind::NotFound)
    })
    .map(drop)
  }

  /// Explicit source selection leaves unrelated same-name data intact.
  #[test]
  fn test_explicit_source() -> Result<(), PredicateFailure<ExplicitSource>> {
    let error = ExplicitSource {
      source: "display data".to_owned(),
      io:     IoError::from(ErrorKind::PermissionDenied),
    };
    ensure_that(
      error,
      "explicit source preserves both native I/O and the unrelated source field",
      |subject| {
        subject.source == "display data"
          && subject.to_string() == "explicit source"
          && subject
            .source()
            .and_then(|cause| cause.downcast_ref::<IoError>())
            .is_some_and(|cause| cause.kind() == ErrorKind::PermissionDenied)
      },
    )
    .map(drop)
  }

  /// Boxed standard error objects remain usable as concrete sources.
  #[test]
  fn test_boxed_source() -> Result<(), PredicateFailure<BoxedSource>> {
    let error = BoxedSource {
      source: Box::new(IoError::from(ErrorKind::Interrupted)),
    };
    ensure_that(error, "boxed source retains the native I/O error", |subject| {
      subject.to_string() == "boxed source"
        && subject
          .source()
          .and_then(|cause| cause.downcast_ref::<IoError>())
          .is_some_and(|cause| cause.kind() == ErrorKind::Interrupted)
    })
    .map(drop)
  }

  /// The surrounding macro does not change source or conversion hygiene.
  #[test]
  fn test_macro_source() -> Result<(), PredicateFailure<MacroSource>> {
    let error = MacroSource::from(IoError::from(ErrorKind::NotFound));
    ensure_that(error, "macro-generated conversion retains its native source", |subject| {
      subject.to_string() == "Something"
        && subject
          .source()
          .and_then(|cause| cause.downcast_ref::<IoError>())
          .is_some_and(|cause| cause.kind() == ErrorKind::NotFound)
    })
    .map(drop)
  }

  /// Non-error data with the conventional name does not become a source.
  #[test]
  fn test_not_source() -> Result<(), PredicateFailure<NotSource>> {
    let error = NotSource {
      source:      'S',
      destination: 'D',
    };
    ensure_that(
      error,
      "non-error source data remains formatting data without an error source",
      |subject| subject.to_string() == "S ==> D" && subject.source().is_none(),
    )
    .map(drop)
  }

  /// Raw spelling distinguishes display data from conventional source selection.
  #[derive(Debug, Error)]
  #[error("{source} ==> {destination}")]
  struct NotSource {
    /// Original source character.
    r#source:    char,
    /// Original destination character.
    destination: char,
  }
}
