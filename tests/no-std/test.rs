#![no_std]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
  #[error("Error::E")]
  E(#[from] SourceError),
}

#[derive(Error, Debug)]
#[error("SourceError {field}")]
pub struct SourceError {
  pub field: i32,
}

#[cfg(test)]
mod tests {
  use core::error::Error as _;
  use core::fmt::Write;
  use core::fmt::{
    self,
  };
  use core::mem;

  use strict_test_support::ComparisonFailure;
  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_eq;
  use strict_test_support::ensure_that;

  use crate::Error;
  use crate::SourceError;

  /// A rejected comparison retaining formatting outcomes and both complete buffers.
  type BufferComparison<Outcome, const N: usize> = ComparisonFailure<(Outcome, [u8; N]), (Outcome, [u8; N])>;

  /// Typed failures from checking error sources and allocation-free formatting.
  #[derive(Debug, thiserror::Error)]
  enum NoStdTestFailure {
    /// The derived source chain failed to expose the expected concrete error.
    #[error(transparent)]
    Source(#[from] PredicateFailure<Error>),
    /// The enum's formatting result or complete output buffer differed.
    #[error(transparent)]
    EnumDisplay(#[from] BufferComparison<fmt::Result, 17>),
    /// The source's presence, formatting result, or complete output buffer differed.
    #[error(transparent)]
    SourceDisplay(#[from] BufferComparison<Option<fmt::Result>, 17>),
  }

  struct Buf<'a>(&'a mut [u8]);

  impl Write for Buf<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
      if s.len() > self.0.len() {
        return Err(fmt::Error);
      }
      let Some((out, rest)) = mem::take(&mut self.0).split_at_mut_checked(s.len()) else {
        return Err(fmt::Error);
      };
      for (slot, byte) in out.iter_mut().zip(s.bytes()) {
        *slot = byte;
      }
      self.0 = rest;
      Ok(())
    }
  }

  #[test]
  fn test() -> Result<(), NoStdTestFailure> {
    let source = SourceError {
      field: -1
    };
    let error = Error::from(source);

    let error = ensure_that(error, "the enum error exposes its #[from] source", |error| error.source().is_some())?;
    let error = ensure_that(error, "the exposed source downcasts to SourceError", |error| {
      error
        .source()
        .and_then(|source| source.downcast_ref::<SourceError>())
        .is_some_and(|source| source.field == -1)
    })?;

    let mut msg = [b'~'; 17];
    let rendered = write!(Buf(&mut msg), "{error}");
    ensure_eq(
      (rendered, msg),
      (Ok(()), *b"Error::E~~~~~~~~~"),
      "the enum display is preserved without std",
    )
    .map(drop)?;

    let mut msg = [b'~'; 17];
    let rendered = error.source().map(|source| write!(Buf(&mut msg), "{source}"));
    ensure_eq(
      (rendered, msg),
      (Some(Ok(())), *b"SourceError -1~~~"),
      "the source display is preserved without std",
    )
    .map(drop)
    .map_err(NoStdTestFailure::from)
  }

  #[test]
  fn source_has_no_nested_source() -> Result<(), PredicateFailure<SourceError>> {
    ensure_that(
      SourceError {
        field: -1
      },
      "the leaf source error has no nested source",
      |source| source.source().is_none(),
    )
    .map(drop)
  }

  #[test]
  fn display_propagates_buffer_exhaustion() -> Result<(), BufferComparison<fmt::Result, 7>> {
    let error = Error::from(SourceError {
      field: -1
    });
    let mut msg = [b'~'; 7];
    let rendered = write!(Buf(&mut msg), "{error}");
    ensure_eq(
      (rendered, msg),
      (Err(fmt::Error), [b'~'; 7]),
      "the derived display propagates an undersized buffer's error without writing",
    )
    .map(drop)
  }
}
