#![allow(clippy::needless_late_init, clippy::uninlined_format_args)]

use core::fmt::Debug;
use core::fmt::Display;
use core::fmt::{
  self,
};
use core::str::FromStr;

use strict_test_support::ComparisonFailure;
use strict_test_support::ensure_eq;
use thiserror::Error;

pub struct NoFormat;

#[derive(Debug)]
pub struct DebugOnly;

pub struct DisplayOnly;

impl Display for DisplayOnly {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("display only")
  }
}

#[derive(Debug)]
pub struct DebugAndDisplay;

impl Display for DebugAndDisplay {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("debug and display")
  }
}

// Should expand to:
//
//     impl<E> Display for EnumDebugField<E>
//     where
//         E: Debug;
//
//     impl<E> Error for EnumDebugField<E>
//     where
//         Self: Debug + Display;
//
#[derive(Error, Debug)]
pub enum EnumDebugGeneric<E> {
  #[error("{0:?}")]
  FatalError(E),
}

// Should expand to:
//
//     impl<E> Display for EnumFromGeneric<E>;
//
//     impl<E> Error for EnumFromGeneric<E>
//     where
//         EnumDebugGeneric<E>: Error + 'static,
//         Self: Debug + Display;
//
#[derive(Error, Debug)]
pub enum EnumFromGeneric<E> {
  #[error("enum from generic")]
  Source(#[from] EnumDebugGeneric<E>),
}

// Should expand to:
//
//     impl<HasDisplay, HasDebug, HasNeither> Display
//         for EnumCompound<HasDisplay, HasDebug, HasNeither>
//     where
//         HasDisplay: Display,
//         HasDebug: Debug;
//
//     impl<HasDisplay, HasDebug, HasNeither> Error
//         for EnumCompound<HasDisplay, HasDebug, HasNeither>
//     where
//         Self: Debug + Display;
//
#[derive(Error)]
pub enum EnumCompound<HasDisplay, HasDebug, HasNeither> {
  #[error("{0} {1:?}")]
  DisplayDebug(HasDisplay, HasDebug),
  #[error("{0}")]
  Display(HasDisplay, HasNeither),
  #[error("{1:?}")]
  Debug(HasNeither, HasDebug),
}

impl<HasDisplay, HasDebug, HasNeither> Debug for EnumCompound<HasDisplay, HasDebug, HasNeither> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("EnumCompound")
  }
}

#[test]
fn test_display_enum_compound() -> Result<(), ComparisonFailure<String, String>> {
  let mut instance: EnumCompound<DisplayOnly, DebugOnly, NoFormat>;

  instance = EnumCompound::DisplayDebug(DisplayOnly, DebugOnly);
  ensure_eq(
    instance.to_string(),
    "display only DebugOnly".to_owned(),
    "a {0} {1:?} variant renders display then debug",
  )?;

  instance = EnumCompound::Display(DisplayOnly, NoFormat);
  ensure_eq(
    instance.to_string(),
    "display only".to_owned(),
    "a {0} variant renders only the display field",
  )?;

  instance = EnumCompound::Debug(NoFormat, DebugOnly);
  ensure_eq(
    instance.to_string(),
    "DebugOnly".to_owned(),
    "a {1:?} variant renders only the debug field",
  )
  .map(drop)
}

// Should expand to:
//
//     impl<E> Display for EnumTransparentGeneric<E>
//     where
//         E: Display;
//
//     impl<E> Error for EnumTransparentGeneric<E>
//     where
//         E: Error,
//         Self: Debug + Display;
//
#[derive(Error, Debug)]
pub enum EnumTransparentGeneric<E> {
  #[error(transparent)]
  Other(E),
}

// Should expand to:
//
//     impl<E> Display for StructDebugGeneric<E>
//     where
//         E: Debug;
//
//     impl<E> Error for StructDebugGeneric<E>
//     where
//         Self: Debug + Display;
//
#[derive(Error, Debug)]
#[error("{underlying:?}")]
pub struct StructDebugGeneric<E> {
  pub underlying: E,
}

// Should expand to:
//
//     impl<E> Error for StructFromGeneric<E>
//     where
//         StructDebugGeneric<E>: Error + 'static,
//         Self: Debug + Display;
//
#[derive(Error, Debug)]
pub struct StructFromGeneric<E> {
  #[from]
  pub source: StructDebugGeneric<E>,
}

// Should expand to:
//
//     impl<E> Display for StructTransparentGeneric<E>
//     where
//         E: Display;
//
//     impl<E> Error for StructTransparentGeneric<E>
//     where
//         E: Error,
//         Self: Debug + Display;
//
#[derive(Error, Debug)]
#[error(transparent)]
pub struct StructTransparentGeneric<E>(pub E);

// Should expand to:
//
//     impl<T: FromStr> Display for AssociatedTypeError<T>
//     where
//         T::Err: Display;
//
//     impl<T: FromStr> Error for AssociatedTypeError<T>
//     where
//         Self: Debug + Display;
//
#[derive(Error, Debug)]
pub enum AssociatedTypeError<T: FromStr> {
  #[error("couldn't parse matrix")]
  Other,
  #[error("couldn't parse entry: {0}")]
  EntryParseError(T::Err),
}

// Regression test for https://github.com/dtolnay/thiserror/issues/345
#[test]
fn test_no_bound_on_named_fmt() -> Result<(), ComparisonFailure<String, String>> {
  #[derive(Error, Debug)]
  #[error("{thing}", thing = "...")]
  struct Error<T> {
    thing: T,
  }

  let error = Error {
    thing: DebugOnly
  };
  ensure_eq(
    error.to_string(),
    "...".to_owned(),
    "a user-written named argument shadows the field without bounding T",
  )
  .map(drop)
}

#[test]
fn test_multiple_bound() -> Result<(), ComparisonFailure<String, String>> {
  #[derive(Error, Debug)]
  #[error("0x{thing:x} 0x{thing:X}")]
  pub struct Error<T> {
    thing: T,
  }

  let error = Error {
    thing: 0xFFi32
  };
  ensure_eq(
    error.to_string(),
    "0xff 0xFF".to_owned(),
    "one field renders through both hex trait bounds",
  )
  .map(drop)
}
