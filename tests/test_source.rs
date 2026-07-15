use std::error::Error as StdError;
use std::io;

use strict_test_support::TestFailure;
use strict_test_support::ensure;
use thiserror::Error;

mod support;

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
fn test_implicit_source() -> Result<(), TestFailure> {
  let io = io::Error::other("oh no!");
  let error = ImplicitSource {
    source: io
  };
  ensure_display(&error, "implicit source", "implicit source display is preserved")?;
  let source = ensure_source::<io::Error>(&error, "implicit source is an io::Error")?;
  ensure_display(source, "oh no!", "implicit source message is preserved")
}

#[test]
fn test_explicit_source() -> Result<(), TestFailure> {
  let io = io::Error::other("oh no!");
  let error = ExplicitSource {
    source: String::new(),
    io,
  };
  ensure_display(&error, "explicit source", "explicit source display is preserved")?;
  let source = ensure_source::<io::Error>(&error, "explicit source is an io::Error")?;
  ensure_display(source, "oh no!", "explicit source message is preserved")
}

#[test]
fn test_boxed_source() -> Result<(), TestFailure> {
  let source = Box::new(io::Error::other("oh no!"));
  let error = BoxedSource {
    source,
  };
  ensure_display(&error, "boxed source", "boxed source display is preserved")?;
  let source = ensure_source::<io::Error>(&error, "boxed source is an io::Error")?;
  ensure_display(source, "oh no!", "boxed source message is preserved")
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
fn test_macro_source() -> Result<(), TestFailure> {
  let error = MacroSource::from(io::Error::other("macro source"));
  ensure_display(&error, "Something", "macro-generated display is usable")?;
  let source = ensure_source::<io::Error>(&error, "macro-generated source is an io::Error")?;
  ensure_display(source, "macro source", "macro-generated source message is preserved")
}

#[test]
fn test_not_source() -> Result<(), TestFailure> {
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
  ensure(
    error.source().is_none(),
    "a non-error field named source does not become an error source",
  )
}
