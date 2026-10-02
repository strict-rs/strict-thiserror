//! Error-only derives preserve caller-defined Display implementations and concrete sources.

/// The derive supplies Error while each caller owns its display contract.
#[cfg(test)]
mod tests {
  use core::fmt::Display;
  use core::fmt::Formatter;
  use core::fmt::Result as FormatResult;
  use std::error::Error as StdError;
  use std::io::Error as IoError;
  use std::io::ErrorKind;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// Named fields remain native diagnostic data when display is caller-owned.
  #[derive(Debug, Error)]
  struct BracedError {
    /// Original message.
    msg: String,
    /// Original position.
    pos: usize,
  }

  impl Display for BracedError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
      write!(formatter, "{}:{}", self.msg, self.pos)
    }
  }

  /// Numbered fields remain native diagnostic data when display is caller-owned.
  #[derive(Debug, Error)]
  struct TupleError(String, usize);

  impl Display for TupleError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
      write!(formatter, "{}:{}", self.0, self.1)
    }
  }

  /// Unit errors retain a caller-provided literal display.
  #[derive(Debug, Error)]
  struct UnitError;

  /// An explicit source remains native when the derive does not supply display.
  #[derive(Debug, Error)]
  struct WithSource {
    /// Original concrete source.
    #[source]
    cause: IoError,
  }

  /// Anyhow interoperability remains a test-only concrete source contract.
  #[derive(Debug, Error)]
  struct WithAnyhow {
    /// Original context-bearing error.
    #[source]
    cause: anyhow::Error,
  }

  /// The caller owns display across all supported enum field shapes.
  #[derive(Debug, Error)]
  enum EnumError {
    /// Named native source.
    Braced {
      /// Original concrete source.
      #[source]
      cause: IoError,
    },
    /// Numbered native source.
    Tuple(#[source] IoError),
    /// Source-free variant.
    Unit,
  }

  impl Display for EnumError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
      formatter.write_str(match *self {
        Self::Braced {
          ..
        } => "Braced",
        Self::Tuple(_) => "Tuple",
        Self::Unit => "Unit",
      })
    }
  }

  /// Literal display is a real caller contract for payload-insensitive source wrappers.
  macro_rules! literal_display {
    ($($subject:ty => $message:literal),+ $(,)?) => {
      $(
        impl Display for $subject {
          fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
            formatter.write_str($message)
          }
        }
      )+
    };
  }

  literal_display! { UnitError => "UnitError", WithSource => "WithSource", WithAnyhow => "WithAnyhow" }

  /// Original named, numbered, and unit error owners with caller-defined displays.
  type StructOwners = (BracedError, TupleError, UnitError);

  /// Error-only derives do not replace native fields or the caller's display implementation.
  #[test]
  fn caller_display_preserves_struct_field_shapes() -> Result<(), PredicateFailure<StructOwners>> {
    let owners = (
      BracedError {
        msg: "named".to_owned(),
        pos: 3,
      },
      TupleError("numbered".to_owned(), 4),
      UnitError,
    );
    ensure_that(owners, "caller-owned display remains independent of the Error derive", |observed| {
      observed.0.to_string() == "named:3"
        && observed.0.source().is_none()
        && observed.1.to_string() == "numbered:4"
        && observed.1.source().is_none()
        && observed.2.to_string() == "UnitError"
        && observed.2.source().is_none()
    })
    .map(drop)
  }

  /// Concrete sources stay attached to their original caller-displayed owners.
  #[test]
  fn caller_display_preserves_concrete_sources() -> Result<(), PredicateFailure<(WithSource, WithAnyhow)>> {
    let owners = (
      WithSource {
        cause: IoError::from(ErrorKind::NotFound),
      },
      WithAnyhow {
        cause: anyhow::Error::new(IoError::from(ErrorKind::PermissionDenied)),
      },
    );
    ensure_that(
      owners,
      "Error-only source selection retains typed standard and anyhow errors",
      |observed| {
        observed.0.to_string() == "WithSource"
          && observed
            .0
            .source()
            .and_then(|cause| cause.downcast_ref::<IoError>())
            .is_some_and(|cause| cause.kind() == ErrorKind::NotFound)
          && observed.1.to_string() == "WithAnyhow"
          && observed
            .1
            .source()
            .and_then(|cause| cause.downcast_ref::<IoError>())
            .is_some_and(|cause| cause.kind() == ErrorKind::PermissionDenied)
      },
    )
    .map(drop)
  }

  /// All enum field shapes retain native source presence and absence.
  #[test]
  fn caller_display_preserves_enum_source_shapes() -> Result<(), PredicateFailure<[EnumError; 3]>> {
    let owners = [
      EnumError::Braced {
        cause: IoError::from(ErrorKind::NotFound),
      },
      EnumError::Tuple(IoError::from(ErrorKind::Interrupted)),
      EnumError::Unit,
    ];
    ensure_that(
      owners,
      "Error-only enums preserve caller display and every concrete source shape",
      |observed| {
        observed
          .iter()
          .zip(["Braced", "Tuple", "Unit"])
          .all(|(subject, expected)| subject.to_string() == expected)
          && observed
            .iter()
            .filter_map(StdError::source)
            .filter_map(|cause| cause.downcast_ref::<IoError>())
            .map(IoError::kind)
            .eq([ErrorKind::NotFound, ErrorKind::Interrupted])
          && observed.iter().filter(|subject| subject.source().is_none()).count() == 1
      },
    )
    .map(drop)
  }
}
