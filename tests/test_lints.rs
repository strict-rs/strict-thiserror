//! Derives remain compatible with strict caller lint policy and borrowed conversions.

/// Caller lint contracts apply to the complete concrete test owners.
#[cfg(test)]
mod tests {
  use std::error::Error;
  use std::io::Error as IoError;
  use std::io::ErrorKind;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// A unit derive does not require qualifications to be suppressed.
  #[derive(Debug, Error)]
  #[error("caller lint contract")]
  struct CallerError;

  /// Generic lifetime parameters stay attached to the original borrowed data.
  #[derive(Debug, Error)]
  enum BorrowedError<'a> {
    /// Native conversion source.
    #[error("I/O")]
    Io(#[from] IoError),
    /// Original non-source borrow.
    #[error("{0}")]
    Borrowed(&'a str),
  }

  /// Strict callers can use a generated error without any allow attributes.
  #[test]
  fn test_allow_attributes() -> Result<(), PredicateFailure<CallerError>> {
    ensure_that(
      CallerError,
      "the generated error remains usable under strict caller policy",
      |subject| subject.to_string() == "caller lint contract" && subject.source().is_none(),
    )
    .map(drop)
  }

  /// Importing the standard Error trait does not make generated qualifications a caller error.
  #[test]
  fn test_unused_qualifications() -> Result<(), PredicateFailure<CallerError>> {
    ensure_that(
      CallerError,
      "caller trait imports retain generated source and display behavior",
      |subject| Error::source(subject).is_none() && subject.to_string() == "caller lint contract",
    )
    .map(drop)
  }

  /// Borrowed variants keep their original data while conversion variants keep native sources.
  #[test]
  fn test_needless_lifetimes() -> Result<(), PredicateFailure<String>> {
    let text = String::from("borrowed");
    ensure_that(text, "generated implementations retain the native borrow lifetime", |subject| {
      let borrowed = BorrowedError::Borrowed(subject.as_str());
      let converted = BorrowedError::from(IoError::from(ErrorKind::NotFound));
      borrowed.to_string() == subject.as_str()
        && borrowed.source().is_none()
        && converted
          .source()
          .and_then(|cause| cause.downcast_ref::<IoError>())
          .is_some_and(|cause| cause.kind() == ErrorKind::NotFound)
    })
    .map(drop)
  }

  /// An ordinary conversion does not introduce explicit unnecessary lifetime parameters.
  #[test]
  fn test_forbid_needless_lifetimes() -> Result<(), PredicateFailure<BorrowedError<'static>>> {
    let owner = BorrowedError::from(IoError::from(ErrorKind::Interrupted));
    ensure_that(
      owner,
      "conversion retains its native source under the inherited lifetime policy",
      |subject| {
        subject.to_string() == "I/O"
          && subject
            .source()
            .and_then(|cause| cause.downcast_ref::<IoError>())
            .is_some_and(|cause| cause.kind() == ErrorKind::Interrupted)
      },
    )
    .map(drop)
  }

  /// Full rustc integration owns compatibility with deprecated declaration syntax.
  #[test]
  fn test_deprecated() -> Result<(), trybuild::TryBuildError> {
    let mut cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/deprecated-declarations.rs");
    cases.compile_fail("tests/ui/source-struct-not-error.rs");
    cases.run()
  }
}
