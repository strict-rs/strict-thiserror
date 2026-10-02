use core::error::Error;
use core::panic::UnwindSafe;

#[doc(hidden)]
/// Borrow a concrete source through the standard error trait used by generated implementations.
pub trait AsDynError<'a>: Sealed {
  /// Retain the original source while exposing its standard error contract.
  fn as_dyn_error(&self) -> &(dyn Error + 'a);
}

impl<'a, T: Error + 'a> AsDynError<'a> for T {
  #[inline]
  fn as_dyn_error(&self) -> &(dyn Error + 'a) {
    self
  }
}

impl<'a> AsDynError<'a> for dyn Error + 'a {
  #[inline]
  fn as_dyn_error(&self) -> &(dyn Error + 'a) {
    self
  }
}

impl<'a> AsDynError<'a> for dyn Error + Send + 'a {
  #[inline]
  fn as_dyn_error(&self) -> &(dyn Error + 'a) {
    self
  }
}

impl<'a> AsDynError<'a> for dyn Error + Send + Sync + 'a {
  #[inline]
  fn as_dyn_error(&self) -> &(dyn Error + 'a) {
    self
  }
}

impl<'a> AsDynError<'a> for dyn Error + Send + Sync + UnwindSafe + 'a {
  #[inline]
  fn as_dyn_error(&self) -> &(dyn Error + 'a) {
    self
  }
}

#[doc(hidden)]
/// Restrict dynamic-source conversion to supported standard error subjects.
pub trait Sealed {}
impl<T: Error> Sealed for T {}
impl Sealed for dyn Error + '_ {}
impl Sealed for dyn Error + Send + '_ {}
impl Sealed for dyn Error + Send + Sync + '_ {}
impl Sealed for dyn Error + Send + Sync + UnwindSafe + '_ {}

/// Dynamic error views preserve native error identity and source absence.
#[cfg(test)]
mod tests {
  use core::error::Error;
  use core::num::ParseIntError;
  use core::panic::UnwindSafe;
  use core::ptr;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;

  use super::AsDynError as _;

  /// Complete native parser outcome used to obtain a concrete error.
  type ParsedNumber = Result<u64, ParseIntError>;

  /// Plain dynamic sources retain the same concrete error and do not invent a source.
  #[test]
  fn plain_dynamic_errors_preserve_identity() -> Result<(), PredicateFailure<ParsedNumber>> {
    ensure_that(
      "invalid".parse(),
      "plain dynamic conversion retains the original parser error",
      |observed| {
        let Err(ref source) = *observed else { return false };
        let dynamic: &dyn Error = source;
        let converted = dynamic.as_dyn_error();
        (
          converted
            .downcast_ref::<ParseIntError>()
            .is_some_and(|original| ptr::eq(original, source)),
          converted.source().is_none(),
        ) == (true, true)
      },
    )
    .map(drop)
  }

  /// Additional auto traits do not replace the underlying error or fabricate a source.
  #[test]
  fn unwind_safe_dynamic_errors_preserve_identity() -> Result<(), PredicateFailure<ParsedNumber>> {
    ensure_that(
      "invalid".parse(),
      "auto-trait conversion retains the original parser error",
      |observed| {
        let Err(ref source) = *observed else { return false };
        let dynamic: &(dyn Error + Send + Sync + UnwindSafe) = source;
        let converted = dynamic.as_dyn_error();
        (
          converted
            .downcast_ref::<ParseIntError>()
            .is_some_and(|original| ptr::eq(original, source)),
          converted.source().is_none(),
        ) == (true, true)
      },
    )
    .map(drop)
  }
}
