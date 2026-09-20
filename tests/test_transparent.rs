use std::error::Error as StdError;
use std::io;

use strict_test_support::ComparisonFailure;
use strict_test_support::PredicateFailure;
use strict_test_support::ensure_that;
use thiserror::Error;

mod support;

use support::SourceFailure;
use support::ensure_display;
use support::ensure_source;

#[test]
fn test_transparent_struct() -> Result<(), impl StdError> {
  #[derive(Error, Debug)]
  #[error(transparent)]
  struct Error(ErrorKind);

  #[derive(Error, Debug)]
  enum ErrorKind {
    #[error("E0")]
    E0,
    #[error("E1")]
    E1(#[from] io::Error),
  }

  let error = Error(ErrorKind::E0);
  ensure_display(&error, "E0", "transparent struct delegates display")?;
  ensure_that(error, "source-absent transparent struct remains source-absent", |error| {
    StdError::source(error).is_none()
  })
  .map(drop)?;

  let io = io::Error::other("oh no!");
  let error = Error(ErrorKind::from(io));
  ensure_display(&error, "E1", "transparent struct delegates variant display")?;
  ensure_source::<io::Error, _>(
    error,
    "oh no!",
    "transparent struct preserves its nested io::Error source",
    "transparent struct preserves its nested source message",
  )
}

#[test]
fn test_transparent_enum() -> Result<(), impl StdError> {
  #[derive(Error, Debug)]
  enum Error {
    #[error("this failed")]
    This,
    #[error(transparent)]
    Other(anyhow::Error),
  }

  let error = Error::This;
  ensure_display(&error, "this failed", "ordinary enum variant preserves display")?;
  ensure_that(error, "ordinary enum variant remains source-absent", |error| {
    StdError::source(error).is_none()
  })
  .map(drop)?;

  let error = Error::Other(anyhow::Error::new(io::Error::other("inner")).context("outer"));
  ensure_display(&error, "outer", "transparent enum delegates anyhow context display")?;
  ensure_source::<io::Error, _>(
    error,
    "inner",
    "transparent enum preserves the anyhow source chain",
    "transparent enum exposes the inner source message",
  )
}

#[test]
fn test_transparent_enum_with_default_message() -> Result<(), impl StdError> {
  #[derive(Error, Debug)]
  #[error("this failed: {0}_{1}")]
  enum Error {
    This(i32, i32),
    #[error(transparent)]
    Other(anyhow::Error),
  }

  let error = Error::This(-1, -1);
  ensure_display(&error, "this failed: -1_-1", "enum default display applies to ordinary variants")?;
  ensure_that(error, "default-message variant remains source-absent", |error| {
    StdError::source(error).is_none()
  })
  .map(drop)?;

  let error = Error::Other(anyhow::Error::new(io::Error::other("inner")).context("outer"));
  ensure_display(&error, "outer", "transparent variant overrides the enum default display")?;
  ensure_source::<io::Error, _>(
    error,
    "inner",
    "transparent variant preserves the anyhow source chain",
    "transparent variant exposes the inner source message",
  )
}

#[test]
fn test_transparent_enum_generic() -> Result<(), impl StdError> {
  #[derive(Error, Debug)]
  enum Error<E> {
    #[error("this failed")]
    This,
    #[error(transparent)]
    Other(E),
  }

  #[derive(Error, Debug)]
  #[error("inner error")]
  struct Inner;

  #[derive(Error, Debug)]
  #[error("wrapped")]
  struct WithSource(#[source] io::Error);

  /// Native failures from checking the distinct generic instantiations.
  #[derive(Debug, thiserror::Error)]
  enum GenericTestFailure {
    /// A generic error's display differed.
    #[error(transparent)]
    Display(#[from] ComparisonFailure<String, String>),
    /// A source-free instantiation unexpectedly exposed a source.
    #[error(transparent)]
    AbsentSource(#[from] PredicateFailure<Error<Inner>>),
    /// A source-bearing instantiation failed its source contract.
    #[error(transparent)]
    PresentSource(#[from] SourceFailure<Error<WithSource>>),
  }

  let error = Error::<Inner>::This;
  ensure_display(&error, "this failed", "generic enum ordinary variant preserves display")?;
  ensure_that(error, "generic enum ordinary variant remains source-absent", |error| {
    StdError::source(error).is_none()
  })
  .map(drop)?;

  let error = Error::Other(Inner);
  ensure_display(&error, "inner error", "generic transparent variant delegates display")?;
  ensure_that(error, "generic transparent variant preserves an absent source", |error| {
    StdError::source(error).is_none()
  })
  .map(drop)?;

  let io = io::Error::other("oh no!");
  let error = Error::Other(WithSource(io));
  ensure_display(&error, "wrapped", "generic transparent variant delegates wrapped display")?;
  ensure_source::<io::Error, _>(
    error,
    "oh no!",
    "generic transparent variant preserves its typed source",
    "generic transparent variant preserves its source message",
  )
  .map_err(GenericTestFailure::from)
}

#[test]
fn test_anyhow() -> Result<(), impl StdError> {
  #[derive(Error, Debug)]
  #[error(transparent)]
  struct Any(#[from] anyhow::Error);

  let error = Any::from(anyhow::Error::new(io::Error::other("inner")).context("outer"));
  ensure_display(&error, "outer", "transparent anyhow struct delegates context display")?;
  ensure_source::<io::Error, _>(
    error,
    "inner",
    "transparent anyhow struct preserves its source chain",
    "transparent anyhow struct exposes the inner source message",
  )
}

#[test]
fn test_non_static() -> Result<(), impl StdError> {
  #[derive(Error, Debug)]
  #[error(transparent)]
  struct Error<'a> {
    inner: ErrorKind<'a>,
  }

  #[derive(Error, Debug)]
  enum ErrorKind<'a> {
    #[error("unexpected token: {:?}", token)]
    Unexpected { token: &'a str },
  }

  let error = Error {
    inner: ErrorKind::Unexpected {
      token: "error"
    },
  };
  ensure_display(
    &error,
    "unexpected token: \"error\"",
    "transparent borrowed non-source field preserves display",
  )?;
  ensure_that(error, "transparent borrowed non-source field remains source-absent", |error| {
    StdError::source(error).is_none()
  })
  .map(drop)
  .map_err(SourceFailure::from)
}
