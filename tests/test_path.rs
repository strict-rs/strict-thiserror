//! Owned, borrowed, and combined path formatting preserve their native views.

#![cfg(feature = "std")]

/// Native path owners remain intact through every display comparison.
#[cfg(test)]
mod tests {
  use std::error::Error as _;
  use std::path::Path;
  use std::path::PathBuf;

  use ref_cast::RefCast;
  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// Owned named path field supports shorthand display.
  #[derive(Debug, Error)]
  #[error("failed to read '{file}'")]
  struct StructPathBuf {
    /// Original native path.
    file: PathBuf,
  }

  /// Borrowed unsized path field supports the same shorthand display.
  #[derive(Debug, Error, RefCast)]
  #[repr(C)]
  #[error("failed to read '{file}'")]
  struct StructPath {
    /// Original unsized native path.
    file: Path,
  }

  /// Owned numbered path field supports shorthand display.
  #[derive(Debug, Error)]
  enum EnumPathBuf {
    /// Original native path.
    #[error("failed to read '{0}'")]
    Read(PathBuf),
  }

  /// Both trait orders preserve the path's native debug and display contracts.
  #[derive(Debug, Error)]
  enum BothError {
    /// Display followed by Debug.
    #[error("display:{0} debug:{0:?}")]
    DisplayDebug(PathBuf),
    /// Debug followed by Display.
    #[error("debug:{0:?} display:{0}")]
    DebugDisplay(PathBuf),
  }

  /// Complete source path and both owned error representations.
  type PathOwners = (PathBuf, StructPathBuf, EnumPathBuf);

  /// The same native path renders consistently through owned and unsized borrowed errors.
  #[test]
  fn test_display() -> Result<(), PredicateFailure<PathOwners>> {
    let owners = (
      PathBuf::from("/thiserror"),
      StructPathBuf {
        file: PathBuf::from("/thiserror"),
      },
      EnumPathBuf::Read(PathBuf::from("/thiserror")),
    );
    ensure_that(owners, "owned and borrowed paths preserve the same native rendering", |observed| {
      let borrowed = StructPath::ref_cast(observed.0.as_path());
      observed.1.to_string() == "failed to read '/thiserror'"
        && observed.1.source().is_none()
        && observed.2.to_string() == "failed to read '/thiserror'"
        && observed.2.source().is_none()
        && borrowed.to_string() == "failed to read '/thiserror'"
        && borrowed.source().is_none()
    })
    .map(drop)
  }

  /// Debug and display requirements remain independent of their ordering in the message.
  #[test]
  fn combined_traits_preserve_native_path_views() -> Result<(), PredicateFailure<[BothError; 2]>> {
    let owners = [
      BothError::DisplayDebug(PathBuf::from("path")),
      BothError::DebugDisplay(PathBuf::from("path")),
    ];
    ensure_that(owners, "combined path formatting preserves both native trait views", |observed| {
      observed
        .iter()
        .map(ToString::to_string)
        .eq(["display:path debug:\"path\"", "debug:\"path\" display:path"])
        && observed.iter().all(|subject| subject.source().is_none())
    })
    .map(drop)
  }
}
