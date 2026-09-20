use std::error::Error as StdError;
use std::io;

use strict_test_support::ensure_that;
use thiserror::Error;

mod support;

use support::SourceFailure;
use support::ensure_display;
use support::ensure_source;

#[derive(Error, Debug)]
#[error("implicit source")]
pub struct ImplicitSource {
  source: io::Error,
}

#[derive(Error, Debug)]
#[error("explicit source")]
pub struct ExplicitSource {
  source: String,
  #[source]
  io:     io::Error,
}

#[derive(Error, Debug)]
#[error("boxed source")]
pub struct BoxedSource {
  #[source]
  source: Box<dyn StdError + Send + 'static>,
}

#[test]
fn test_implicit_source() -> Result<(), SourceFailure<ImplicitSource>> {
  let io = io::Error::other("oh no!");
  let error = ImplicitSource {
    source: io
  };
  ensure_display(&error, "implicit source", "implicit source display is preserved")?;
  ensure_source::<io::Error, _>(
    error,
    "oh no!",
    "implicit source is an io::Error",
    "implicit source message is preserved",
  )
}

#[test]
fn test_explicit_source() -> Result<(), SourceFailure<ExplicitSource>> {
  let io = io::Error::other("oh no!");
  let error = ExplicitSource {
    source: String::new(),
    io,
  };
  ensure_display(&error, "explicit source", "explicit source display is preserved")?;
  ensure_source::<io::Error, _>(
    error,
    "oh no!",
    "explicit source is an io::Error",
    "explicit source message is preserved",
  )
}

#[test]
fn test_boxed_source() -> Result<(), SourceFailure<BoxedSource>> {
  let source = Box::new(io::Error::other("oh no!"));
  let error = BoxedSource {
    source,
  };
  ensure_display(&error, "boxed source", "boxed source display is preserved")?;
  ensure_source::<io::Error, _>(error, "oh no!", "boxed source is an io::Error", "boxed source message is preserved")
}

macro_rules! error_from_macro {
    ($($variants:tt)*) => {
        #[derive(Error)]
        #[derive(Debug)]
        pub enum MacroSource {
            $($variants)*
        }
    }
}

// Test that we generate impls with the proper hygiene
#[rustfmt::skip]
error_from_macro! {
    #[error("Something")]
    Variant(#[from] io::Error)
}

#[test]
fn test_macro_source() -> Result<(), SourceFailure<MacroSource>> {
  let error = MacroSource::from(io::Error::other("macro source"));
  ensure_display(&error, "Something", "macro-generated display is usable")?;
  ensure_source::<io::Error, _>(
    error,
    "macro source",
    "macro-generated source is an io::Error",
    "macro-generated source message is preserved",
  )
}

#[test]
fn test_not_source() -> Result<(), impl StdError> {
  #[derive(Error, Debug)]
  #[error("{source} ==> {destination}")]
  pub struct NotSource {
    r#source:    char,
    destination: char,
  }

  let error = NotSource {
    source:      'S',
    destination: 'D',
  };
  ensure_display(&error, "S ==> D", "a field named source remains display data")?;
  ensure_that(error, "a non-error field named source does not become an error source", |error| {
    error.source().is_none()
  })
  .map(drop)
  .map_err(SourceFailure::from)
}
