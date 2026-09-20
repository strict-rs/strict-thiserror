#![cfg(feature = "std")]

use core::fmt::Display;
use std::path::Path;
use std::path::PathBuf;

use ref_cast::RefCast;
use strict_test_support::ComparisonFailure;
use strict_test_support::ensure_eq;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("failed to read '{file}'")]
struct StructPathBuf {
  file: PathBuf,
}

#[derive(Error, Debug, RefCast)]
#[repr(C)]
#[error("failed to read '{file}'")]
struct StructPath {
  file: Path,
}

#[derive(Error, Debug)]
enum EnumPathBuf {
  #[error("failed to read '{0}'")]
  Read(PathBuf),
}

#[derive(Error, Debug)]
#[error("{tail}")]
pub struct UnsizedError {
  pub head: i32,
  pub tail: str,
}

#[derive(Error, Debug)]
pub enum BothError {
  #[error("display:{0} debug:{0:?}")]
  DisplayDebug(PathBuf),
  #[error("debug:{0:?} display:{0}")]
  DebugDisplay(PathBuf),
}

fn ensure_renders<T: Display>(expected: &str, actual: T) -> Result<(), ComparisonFailure<String, String>> {
  ensure_eq(
    actual.to_string(),
    expected.to_owned(),
    "the derived Display output matches the expected rendering",
  )
  .map(drop)
}

#[test]
fn test_display() -> Result<(), ComparisonFailure<String, String>> {
  let path = Path::new("/thiserror");
  let file = path.to_owned();
  ensure_renders("failed to read '/thiserror'", StructPathBuf {
    file,
  })?;
  let file = path.to_owned();
  ensure_renders("failed to read '/thiserror'", EnumPathBuf::Read(file))?;
  ensure_renders("failed to read '/thiserror'", StructPath::ref_cast(path))
}
