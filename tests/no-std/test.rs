//! Concrete error chains and checked formatting buffers work without the standard feature.

#![no_std]

use thiserror::Error;

/// Allocation-free wrapper retaining the concrete source error.
#[derive(Error, Debug, Copy, Clone)]
pub enum NoStdFailure {
  #[error("Error::E")]
  /// A source error is wrapped by the derived conversion.
  Failure(#[from] SourceError),
}

/// A concrete leaf error that formats its signed diagnostic field.
#[derive(Error, Debug, Copy, Clone)]
#[error("SourceError {field}")]
pub struct SourceError {
  /// Native diagnostic field retained by the leaf error.
  pub field: i32,
}

#[cfg(test)]
mod tests {
  use core::error::Error as _;
  use core::fmt::Error as FormatError;
  use core::fmt::Result as FormatResult;
  use core::fmt::Write;
  use core::mem;

  use strict_test_support::ComparisonFailure;
  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_eq;
  use strict_test_support::ensure_that;

  use crate::NoStdFailure;
  use crate::SourceError;

  /// A rejected comparison retaining formatting outcomes and both complete buffers.
  type BufferComparison<Outcome, const N: usize> = ComparisonFailure<(Outcome, [u8; N]), (Outcome, [u8; N])>;

  /// Both accepted native formatting outcomes and their complete fixed-size buffers.
  type AcceptedBuffers<Outcome, const N: usize> = ((Outcome, [u8; N]), (Outcome, [u8; N]));

  /// Typed failures from checking error sources and allocation-free formatting.
  #[derive(Debug, thiserror::Error)]
  enum NoStdTestFailure {
    /// The derived source chain failed to expose the expected concrete error.
    #[error(transparent)]
    Source(#[from] PredicateFailure<NoStdFailure>),
    /// The enum's formatting result or complete output buffer differed.
    #[error(transparent)]
    EnumDisplay(#[from] BufferComparison<FormatResult, 17>),
    /// The source's presence, formatting result, or complete output buffer differed.
    #[error("source formatting for {subject} failed after enum observations {display:?}: {source}")]
    SourceDisplay {
      /// Complete concrete source owner already checked by the predicate assertions.
      subject: NoStdFailure,
      /// Both prior enum formatting observations and their complete buffers.
      display: AcceptedBuffers<FormatResult, 17>,
      /// Native source formatting comparison failure.
      source:  BufferComparison<Option<FormatResult>, 17>,
    },
  }

  /// Write bytes only when the complete string fits the remaining native buffer.
  struct Buf<'a>(&'a mut [u8]);

  impl Write for Buf<'_> {
    fn write_str(&mut self, text: &str) -> FormatResult {
      if text.len() > self.0.len() {
        return Err(FormatError);
      }
      let Some((out, rest)) = mem::take(&mut self.0).split_at_mut_checked(text.len()) else {
        return Err(FormatError);
      };
      for (slot, byte) in out.iter_mut().zip(text.bytes()) {
        *slot = byte;
      }
      self.0 = rest;
      Ok(())
    }
  }

  /// The complete native source and both checked formatting outcomes remain observable.
  #[test]
  fn test() -> Result<(), NoStdTestFailure> {
    let source = SourceError {
      field: -1
    };
    let error = NoStdFailure::from(source);

    let present = ensure_that(error, "the enum error exposes its #[from] source", |subject| {
      subject.source().is_some()
    })?;
    let typed = ensure_that(present, "the exposed source downcasts to SourceError", |subject| {
      subject
        .source()
        .and_then(|cause| cause.downcast_ref::<SourceError>())
        .is_some_and(|leaf| leaf.field == -1)
    })?;

    let mut msg = [b'~'; 17];
    let rendered = write!(Buf(&mut msg), "{typed}");
    let display = ensure_eq(
      (rendered, msg),
      (Ok(()), *b"Error::E~~~~~~~~~"),
      "the enum display is preserved without std",
    )?;

    let mut source_buffer = [b'~'; 17];
    let source_rendered = typed.source().map(|cause| write!(Buf(&mut source_buffer), "{cause}"));
    ensure_eq(
      (source_rendered, source_buffer),
      (Some(Ok(())), *b"SourceError -1~~~"),
      "the source display is preserved without std",
    )
    .map(|source_display| (typed, display, source_display))
    .map(drop)
    .map_err(|failure| NoStdTestFailure::SourceDisplay {
      subject: typed,
      display,
      source: failure,
    })
  }

  /// A concrete leaf retains the absence of a nested source.
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

  /// An undersized buffer fails before writing any bytes.
  #[test]
  fn display_propagates_buffer_exhaustion() -> Result<(), BufferComparison<FormatResult, 7>> {
    let error = NoStdFailure::from(SourceError {
      field: -1
    });
    let mut msg = [b'~'; 7];
    let rendered = write!(Buf(&mut msg), "{error}");
    ensure_eq(
      (rendered, msg),
      (Err(FormatError), [b'~'; 7]),
      "the derived display propagates an undersized buffer's error without writing",
    )
    .map(drop)
  }
}
