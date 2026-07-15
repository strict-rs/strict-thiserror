use std::io;

use strict_test_support::TestFailure;
use strict_test_support::ensure;
use thiserror::Error;

mod support;

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

#[test]
fn test_from() -> Result<(), TestFailure> {
  let error = ErrorStruct::from(io::Error::other("struct source"));
  ensure_display(&error, "...", "struct From conversion preserves display")?;
  let source = ensure_source::<io::Error>(&error, "struct From conversion exposes an io::Error")?;
  ensure_display(source, "struct source", "struct From conversion preserves the source message")?;

  let error = ErrorStructOptional::from(io::Error::other("optional struct source"));
  ensure_display(&error, "...", "optional struct From conversion preserves display")?;
  let source = ensure_source::<io::Error>(&error, "optional struct From conversion exposes an io::Error")?;
  ensure_display(
    source,
    "optional struct source",
    "optional struct From conversion preserves the source message",
  )?;

  let error = ErrorTuple::from(io::Error::other("tuple source"));
  ensure_display(&error, "...", "tuple From conversion preserves display")?;
  let source = ensure_source::<io::Error>(&error, "tuple From conversion exposes an io::Error")?;
  ensure_display(source, "tuple source", "tuple From conversion preserves the source message")?;

  let error = ErrorTupleOptional::from(io::Error::other("optional tuple source"));
  ensure_display(&error, "...", "optional tuple From conversion preserves display")?;
  let source = ensure_source::<io::Error>(&error, "optional tuple From conversion exposes an io::Error")?;
  ensure_display(
    source,
    "optional tuple source",
    "optional tuple From conversion preserves the source message",
  )?;

  let error = ErrorEnum::from(io::Error::other("enum source"));
  ensure_display(&error, "...", "enum From conversion preserves display")?;
  let source = ensure_source::<io::Error>(&error, "enum From conversion exposes an io::Error")?;
  ensure_display(source, "enum source", "enum From conversion preserves the source message")?;

  let error = ErrorEnumOptional::from(io::Error::other("optional enum source"));
  ensure_display(&error, "...", "optional enum From conversion preserves display")?;
  let source = ensure_source::<io::Error>(&error, "optional enum From conversion exposes an io::Error")?;
  ensure_display(
    source,
    "optional enum source",
    "optional enum From conversion preserves the source message",
  )?;

  let error = Many::from(io::Error::other("many io source"));
  ensure(matches!(&error, Many::Io(_)), "io::Error conversion selects Many::Io")?;
  ensure_display(&error, "...", "Many::Io From conversion preserves display")?;
  let source = ensure_source::<io::Error>(&error, "Many::Io exposes an io::Error")?;
  ensure_display(source, "many io source", "Many::Io preserves the source message")?;

  let error = Many::from(anyhow::Error::new(io::Error::other("many anyhow source")));
  ensure(matches!(&error, Many::Any(_)), "anyhow::Error conversion selects Many::Any")?;
  ensure_display(&error, "...", "Many::Any From conversion preserves display")?;
  let source = ensure_source::<io::Error>(&error, "Many::Any exposes its typed underlying source")?;
  ensure_display(source, "many anyhow source", "Many::Any preserves the underlying source message")
}
