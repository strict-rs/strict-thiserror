use std::error::Error as StdError;
use std::io;

use strict_test_support::TestFailure;
use strict_test_support::ensure;
use thiserror::Error;

mod support;

use support::ensure_display;
use support::ensure_source;

#[test]
fn test_transparent_struct() -> Result<(), TestFailure> {
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
  ensure(
    StdError::source(&error).is_none(),
    "source-absent transparent struct remains source-absent",
  )?;

  let io = io::Error::other("oh no!");
  let error = Error(ErrorKind::from(io));
  ensure_display(&error, "E1", "transparent struct delegates variant display")?;
  let source = ensure_source::<io::Error>(&error, "transparent struct preserves its nested io::Error source")?;
  ensure_display(source, "oh no!", "transparent struct preserves its nested source message")
}

#[test]
fn test_transparent_enum() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  enum Error {
    #[error("this failed")]
    This,
    #[error(transparent)]
    Other(anyhow::Error),
  }

  let error = Error::This;
  ensure_display(&error, "this failed", "ordinary enum variant preserves display")?;
  ensure(StdError::source(&error).is_none(), "ordinary enum variant remains source-absent")?;

  let error = Error::Other(anyhow::Error::new(io::Error::other("inner")).context("outer"));
  ensure_display(&error, "outer", "transparent enum delegates anyhow context display")?;
  let source = ensure_source::<io::Error>(&error, "transparent enum preserves the anyhow source chain")?;
  ensure_display(source, "inner", "transparent enum exposes the inner source message")
}

#[test]
fn test_transparent_enum_with_default_message() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("this failed: {0}_{1}")]
  enum Error {
    This(i32, i32),
    #[error(transparent)]
    Other(anyhow::Error),
  }

  let error = Error::This(-1, -1);
  ensure_display(&error, "this failed: -1_-1", "enum default display applies to ordinary variants")?;
  ensure(StdError::source(&error).is_none(), "default-message variant remains source-absent")?;

  let error = Error::Other(anyhow::Error::new(io::Error::other("inner")).context("outer"));
  ensure_display(&error, "outer", "transparent variant overrides the enum default display")?;
  let source = ensure_source::<io::Error>(&error, "transparent variant preserves the anyhow source chain")?;
  ensure_display(source, "inner", "transparent variant exposes the inner source message")
}

#[test]
fn test_transparent_enum_generic() -> Result<(), TestFailure> {
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

  let error = Error::<Inner>::This;
  ensure_display(&error, "this failed", "generic enum ordinary variant preserves display")?;
  ensure(
    StdError::source(&error).is_none(),
    "generic enum ordinary variant remains source-absent",
  )?;

  let error = Error::Other(Inner);
  ensure_display(&error, "inner error", "generic transparent variant delegates display")?;
  ensure(
    StdError::source(&error).is_none(),
    "generic transparent variant preserves an absent source",
  )?;

  let io = io::Error::other("oh no!");
  let error = Error::Other(WithSource(io));
  ensure_display(&error, "wrapped", "generic transparent variant delegates wrapped display")?;
  let source = ensure_source::<io::Error>(&error, "generic transparent variant preserves its typed source")?;
  ensure_display(source, "oh no!", "generic transparent variant preserves its source message")
}

#[test]
fn test_anyhow() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error(transparent)]
  struct Any(#[from] anyhow::Error);

  let error = Any::from(anyhow::Error::new(io::Error::other("inner")).context("outer"));
  ensure_display(&error, "outer", "transparent anyhow struct delegates context display")?;
  let source = ensure_source::<io::Error>(&error, "transparent anyhow struct preserves its source chain")?;
  ensure_display(source, "inner", "transparent anyhow struct exposes the inner source message")
}

#[test]
fn test_non_static() -> Result<(), TestFailure> {
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
  ensure(
    StdError::source(&error).is_none(),
    "transparent borrowed non-source field remains source-absent",
  )
}
