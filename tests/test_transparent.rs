//! Transparent wrappers preserve native messages, source chains, and borrowed data.

/// Both source presence and absence are observable through the original owner.
#[cfg(test)]
mod tests {
  use std::error::Error as StdError;
  use std::io::Error as IoError;
  use std::io::ErrorKind;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// Native leaf and source-bearing variants share one transparent representation.
  #[derive(Debug, Error)]
  enum ErrorKindFixture {
    /// Terminal variant has no source.
    #[error("E0")]
    Leaf,
    /// Concrete source-bearing variant.
    #[error("E1")]
    Source(#[from] IoError),
  }

  /// Transparent struct forwards the representation's display and source.
  #[derive(Debug, Error)]
  #[error(transparent)]
  struct StructWrapper(ErrorKindFixture);

  /// Transparent enum forwards an anyhow context without replacing its typed root cause.
  #[derive(Debug, Error)]
  enum EnumWrapper {
    /// Ordinary source-free variant retains its own message.
    #[error("this failed")]
    This,
    /// Transparent context-bearing variant.
    #[error(transparent)]
    Other(anyhow::Error),
  }

  /// Transparent delegation overrides an enum-level default message.
  #[derive(Debug, Error)]
  #[error("this failed: {0}_{1}")]
  enum DefaultMessage {
    /// Ordinary variant inherits the default format.
    This(i32, i32),
    /// Context-bearing variant delegates its message and source.
    #[error(transparent)]
    Other(anyhow::Error),
  }

  /// Generic delegation does not require every instantiation to expose a source.
  #[derive(Debug, Error)]
  enum GenericWrapper<Source> {
    /// Source-free variant has an independent literal message.
    #[error("this failed")]
    This,
    /// Native generic source owner.
    #[error(transparent)]
    Other(Source),
  }

  /// A typed terminal error has no source to delegate.
  #[derive(Debug, Error)]
  #[error("inner error")]
  struct Inner;

  /// A source-bearing native error retains its concrete I/O error.
  #[derive(Debug, Error)]
  #[error("wrapped")]
  struct WithSource(#[source] IoError);

  /// Complete owners of ordinary, source-free, and source-bearing generic instantiations.
  type GenericOwners = (GenericWrapper<Inner>, GenericWrapper<Inner>, GenericWrapper<WithSource>);

  /// Preserve the exact source kind through any transparent owner.
  fn has_io_source(subject: &impl StdError, expected: ErrorKind) -> bool {
    subject
      .source()
      .and_then(|cause| cause.downcast_ref::<IoError>())
      .is_some_and(|cause| cause.kind() == expected)
  }

  /// Transparent structs retain source absence and forward concrete present sources.
  #[test]
  fn test_transparent_struct() -> Result<(), PredicateFailure<(StructWrapper, StructWrapper)>> {
    let owners = (
      StructWrapper(ErrorKindFixture::Leaf),
      StructWrapper(ErrorKindFixture::Source(IoError::from(ErrorKind::NotFound))),
    );
    ensure_that(
      owners,
      "transparent structs preserve both polarities of source delegation",
      |observed| {
        observed.0.to_string() == "E0"
          && observed.0.source().is_none()
          && observed.1.to_string() == "E1"
          && has_io_source(&observed.1, ErrorKind::NotFound)
      },
    )
    .map(drop)
  }

  /// Transparent enums keep the context message and the typed native root cause.
  #[test]
  fn test_transparent_enum() -> Result<(), PredicateFailure<(EnumWrapper, EnumWrapper)>> {
    let owners = (
      EnumWrapper::This,
      EnumWrapper::Other(anyhow::Error::new(IoError::from(ErrorKind::PermissionDenied)).context("outer")),
    );
    ensure_that(
      owners,
      "transparent enums preserve ordinary messages and context-bearing sources",
      |observed| {
        observed.0.to_string() == "this failed"
          && observed.0.source().is_none()
          && observed.1.to_string() == "outer"
          && has_io_source(&observed.1, ErrorKind::PermissionDenied)
      },
    )
    .map(drop)
  }

  /// The transparent variant's message does not acquire the enum's default format.
  #[test]
  fn test_transparent_enum_with_default_message() -> Result<(), PredicateFailure<(DefaultMessage, DefaultMessage)>> {
    let owners = (
      DefaultMessage::This(-1, -1),
      DefaultMessage::Other(anyhow::Error::new(IoError::from(ErrorKind::Interrupted)).context("outer")),
    );
    ensure_that(
      owners,
      "transparent delegation overrides the default while ordinary variants retain it",
      |observed| {
        observed.0.to_string() == "this failed: -1_-1"
          && observed.0.source().is_none()
          && observed.1.to_string() == "outer"
          && has_io_source(&observed.1, ErrorKind::Interrupted)
      },
    )
    .map(drop)
  }

  /// Distinct generic instantiations preserve source presence and absence independently.
  #[test]
  fn test_transparent_enum_generic() -> Result<(), PredicateFailure<GenericOwners>> {
    let owners = (
      GenericWrapper::<Inner>::This,
      GenericWrapper::Other(Inner),
      GenericWrapper::Other(WithSource(IoError::from(ErrorKind::NotFound))),
    );
    ensure_that(
      owners,
      "generic transparent wrappers retain their complete concrete sources",
      |observed| {
        observed.0.to_string() == "this failed"
          && observed.0.source().is_none()
          && observed.1.to_string() == "inner error"
          && observed.1.source().is_none()
          && observed.2.to_string() == "wrapped"
          && has_io_source(&observed.2, ErrorKind::NotFound)
      },
    )
    .map(drop)
  }

  /// Anyhow conversion retains its concrete root cause through a transparent struct.
  #[test]
  fn test_anyhow() -> Result<(), PredicateFailure<AnyhowWrapper>> {
    let owner = AnyhowWrapper::from(anyhow::Error::new(IoError::from(ErrorKind::PermissionDenied)).context("outer"));
    ensure_that(owner, "anyhow interoperability retains native source identity", |observed| {
      observed.to_string() == "outer" && has_io_source(observed, ErrorKind::PermissionDenied)
    })
    .map(drop)
  }

  /// Native test-only anyhow conversion owner.
  #[derive(Debug, Error)]
  #[error(transparent)]
  struct AnyhowWrapper(#[from] anyhow::Error);

  /// Borrowed non-source data does not acquire an unnecessary static lifetime bound.
  #[derive(Debug, Error)]
  #[error("unexpected token: {token:?}")]
  struct BorrowedKind<'a> {
    /// Original borrowed token.
    token: &'a str,
  }

  /// Transparent wrapper retains the representation's original borrow.
  #[derive(Debug, Error)]
  #[error(transparent)]
  struct BorrowedWrapper<'a> {
    /// Native borrowed representation.
    inner: BorrowedKind<'a>,
  }

  /// Transparent borrowed data retains its original lifetime and has no source.
  #[test]
  fn test_non_static() -> Result<(), PredicateFailure<String>> {
    let text = String::from("error");
    ensure_that(
      text,
      "borrowed transparent data does not require a static source lifetime",
      |observed| {
        let owner = BorrowedWrapper {
          inner: BorrowedKind {
            token: observed.as_str()
          },
        };
        owner.to_string() == "unexpected token: \"error\"" && owner.source().is_none()
      },
    )
    .map(drop)
  }
}
