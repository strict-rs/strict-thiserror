use std::io;

use strict_test_support::ComparisonFailure;
use strict_test_support::PredicateFailure;
use strict_test_support::ensure_that;
use thiserror::Error;

mod support;

use support::SourceFailure;
use support::ensure_display;
use support::ensure_source;

#[derive(Error, Debug)]
#[error("...")]
pub struct ErrorStruct {
  #[from]
  source: io::Error,
}

#[derive(Error, Debug)]
#[error("...")]
pub struct ErrorStructOptional {
  #[from]
  source: Option<io::Error>,
}

#[derive(Error, Debug)]
#[error("...")]
pub struct ErrorTuple(#[from] io::Error);

#[derive(Error, Debug)]
#[error("...")]
pub struct ErrorTupleOptional(#[from] Option<io::Error>);

#[derive(Error, Debug)]
#[error("...")]
pub enum ErrorEnum {
  Test {
    #[from]
    source: io::Error,
  },
}

#[derive(Error, Debug)]
#[error("...")]
pub enum ErrorEnumOptional {
  Test {
    #[from]
    source: Option<io::Error>,
  },
}

#[derive(Error, Debug)]
#[error("...")]
pub enum Many {
  Any(#[from] anyhow::Error),
  Io(#[from] io::Error),
}

/// Complete assertion failures for the supported `From` implementations.
#[derive(Debug, Error)]
enum FromTestFailure {
  /// A converted error's display did not match.
  #[error(transparent)]
  Display(#[from] ComparisonFailure<String, String>),
  /// A conversion selected the wrong enum variant.
  #[error(transparent)]
  Variant(#[from] PredicateFailure<Many>),
  /// A named source was not preserved.
  #[error(transparent)]
  Struct(#[from] SourceFailure<ErrorStruct>),
  /// An optional named source was not preserved.
  #[error(transparent)]
  StructOptional(#[from] SourceFailure<ErrorStructOptional>),
  /// A tuple source was not preserved.
  #[error(transparent)]
  Tuple(#[from] SourceFailure<ErrorTuple>),
  /// An optional tuple source was not preserved.
  #[error(transparent)]
  TupleOptional(#[from] SourceFailure<ErrorTupleOptional>),
  /// An enum source was not preserved.
  #[error(transparent)]
  Enum(#[from] SourceFailure<ErrorEnum>),
  /// An optional enum source was not preserved.
  #[error(transparent)]
  EnumOptional(#[from] SourceFailure<ErrorEnumOptional>),
  /// A source in an enum with multiple conversions was not preserved.
  #[error(transparent)]
  Many(#[from] SourceFailure<Many>),
}

#[test]
fn test_from() -> Result<(), FromTestFailure> {
  let error = ErrorStruct::from(io::Error::other("struct source"));
  ensure_display(&error, "...", "struct From conversion preserves display")?;
  ensure_source::<io::Error, _>(
    error,
    "struct source",
    "struct From conversion exposes an io::Error",
    "struct From conversion preserves the source message",
  )?;

  let error = ErrorStructOptional::from(io::Error::other("optional struct source"));
  ensure_display(&error, "...", "optional struct From conversion preserves display")?;
  ensure_source::<io::Error, _>(
    error,
    "optional struct source",
    "optional struct From conversion exposes an io::Error",
    "optional struct From conversion preserves the source message",
  )?;

  let error = ErrorTuple::from(io::Error::other("tuple source"));
  ensure_display(&error, "...", "tuple From conversion preserves display")?;
  ensure_source::<io::Error, _>(
    error,
    "tuple source",
    "tuple From conversion exposes an io::Error",
    "tuple From conversion preserves the source message",
  )?;

  let error = ErrorTupleOptional::from(io::Error::other("optional tuple source"));
  ensure_display(&error, "...", "optional tuple From conversion preserves display")?;
  ensure_source::<io::Error, _>(
    error,
    "optional tuple source",
    "optional tuple From conversion exposes an io::Error",
    "optional tuple From conversion preserves the source message",
  )?;

  let error = ErrorEnum::from(io::Error::other("enum source"));
  ensure_display(&error, "...", "enum From conversion preserves display")?;
  ensure_source::<io::Error, _>(
    error,
    "enum source",
    "enum From conversion exposes an io::Error",
    "enum From conversion preserves the source message",
  )?;

  let error = ErrorEnumOptional::from(io::Error::other("optional enum source"));
  ensure_display(&error, "...", "optional enum From conversion preserves display")?;
  ensure_source::<io::Error, _>(
    error,
    "optional enum source",
    "optional enum From conversion exposes an io::Error",
    "optional enum From conversion preserves the source message",
  )?;

  let error = Many::from(io::Error::other("many io source"));
  let error = ensure_that(error, "io::Error conversion selects Many::Io", |error| matches!(error, Many::Io(_)))?;
  ensure_display(&error, "...", "Many::Io From conversion preserves display")?;
  ensure_source::<io::Error, _>(
    error,
    "many io source",
    "Many::Io exposes an io::Error",
    "Many::Io preserves the source message",
  )?;

  let error = Many::from(anyhow::Error::new(io::Error::other("many anyhow source")));
  let error = ensure_that(error, "anyhow::Error conversion selects Many::Any", |error| {
    matches!(error, Many::Any(_))
  })?;
  ensure_display(&error, "...", "Many::Any From conversion preserves display")?;
  ensure_source::<io::Error, _>(
    error,
    "many anyhow source",
    "Many::Any exposes its typed underlying source",
    "Many::Any preserves the underlying source message",
  )
  .map_err(FromTestFailure::from)
}
