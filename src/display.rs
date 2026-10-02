use core::fmt::Arguments;
use core::fmt::Display;
#[cfg(feature = "std")]
use std::path;
#[cfg(feature = "std")]
use std::path::Path;
#[cfg(feature = "std")]
use std::path::PathBuf;

#[doc(hidden)]
/// Select a displayable view without requiring path types to implement `Display`.
pub trait AsDisplay<'a>: Sealed {
  /// Displayable view preserving any borrow from the original subject.
  type Target: Display;

  /// Borrow the displayable view used by a shorthand format argument.
  fn as_display(&'a self) -> Self::Target;
}

impl<'a, T> AsDisplay<'a> for &T
where
  T: Display + ?Sized + 'a,
{
  type Target = &'a T;

  fn as_display(&'a self) -> Self::Target {
    *self
  }
}

#[cfg(feature = "std")]
impl<'a> AsDisplay<'a> for Path {
  type Target = path::Display<'a>;

  #[inline]
  fn as_display(&'a self) -> Self::Target {
    self.display()
  }
}

#[cfg(feature = "std")]
impl<'a> AsDisplay<'a> for PathBuf {
  type Target = path::Display<'a>;

  #[inline]
  fn as_display(&'a self) -> Self::Target {
    self.display()
  }
}

#[doc(hidden)]
/// Restrict shorthand display conversion to the views supported by the derive.
pub trait Sealed {}
impl<T: Display + ?Sized> Sealed for &T {}
#[cfg(feature = "std")]
impl Sealed for Path {}
#[cfg(feature = "std")]
impl Sealed for PathBuf {}

// An owned, native formatting view keeps multiple applicable implementations in every feature
// configuration. Inference therefore remains stable when feature unification adds path views.
impl<'a> AsDisplay<'a> for Arguments<'a> {
  type Target = Self;

  #[inline]
  fn as_display(&'a self) -> Self::Target {
    *self
  }
}

impl Sealed for Arguments<'_> {}

/// Formatting views preserve borrowed inputs and propagate native formatting failures.
#[cfg(test)]
#[cfg(feature = "std")]
mod tests {
  use core::fmt;
  use core::fmt::Display;
  use std::string::String;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;

  use super::AsDisplay;

  /// A formatter that rejects output through its real native result contract.
  #[derive(Debug, Copy, Clone)]
  struct FailedFormatting;

  impl Display for FailedFormatting {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
      Err(fmt::Error)
    }
  }

  /// Original borrowed data, complete rendered text, and native formatting completion.
  type FormattingObservation = (String, String, fmt::Result);
  /// Original failing formatter, accepted prefix, and native formatting failure.
  type FailedObservation = (FailedFormatting, String, fmt::Result);

  /// Native formatting arguments retain values borrowed from their original owner.
  #[test]
  fn native_arguments_preserve_borrowed_values() -> Result<(), PredicateFailure<FormattingObservation>> {
    let original = String::from("borrowed");
    let mut rendered = String::new();
    let completion = {
      let arguments = format_args!("original {original}");
      fmt::write(&mut rendered, AsDisplay::as_display(&arguments))
    };
    ensure_that(
      (original, rendered, completion),
      "native arguments retain their original borrowed value",
      |observed| (observed.0.as_str(), observed.1.as_str(), observed.2) == ("borrowed", "original borrowed", Ok(())),
    )
    .map(drop)
  }

  /// A view forwards formatting failure while retaining the successfully written prefix.
  #[test]
  fn native_arguments_preserve_formatting_failure() -> Result<(), PredicateFailure<FailedObservation>> {
    let original = FailedFormatting;
    let mut rendered = String::new();
    let completion = {
      let arguments = format_args!("prefix: {original}");
      fmt::write(&mut rendered, AsDisplay::as_display(&arguments))
    };
    ensure_that(
      (original, rendered, completion),
      "formatting errors retain the original formatter and accepted prefix",
      |observed| (observed.1.as_str(), observed.2) == ("prefix: ", Err(fmt::Error)),
    )
    .map(drop)
  }
}
