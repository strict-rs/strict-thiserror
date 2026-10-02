//! Derived conversions retain concrete source errors and optional wrapping.

/// Exercise every supported conversion shape through its native error owner.
#[cfg(test)]
mod tests {
  use std::error::Error as StdError;
  use std::io::Error as IoError;
  use std::io::ErrorKind;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// Conversion into a named source field.
  #[derive(Debug, Error)]
  #[error("...")]
  struct ErrorStruct {
    /// Native conversion source.
    #[from]
    source: IoError,
  }

  /// Conversion wraps a named optional source in Some.
  #[derive(Debug, Error)]
  #[error("...")]
  struct ErrorStructOptional {
    /// Native optional conversion source.
    #[from]
    source: Option<IoError>,
  }

  /// Conversion into a numbered source field.
  #[derive(Debug, Error)]
  #[error("...")]
  struct ErrorTuple(#[from] IoError);

  /// Conversion wraps a numbered optional source in Some.
  #[derive(Debug, Error)]
  #[error("...")]
  struct ErrorTupleOptional(#[from] Option<IoError>);

  /// Conversion selects a named enum variant.
  #[derive(Debug, Error)]
  #[error("...")]
  enum ErrorEnum {
    /// Native source is retained by the selected variant.
    Test {
      /// Concrete conversion source.
      #[from]
      source: IoError,
    },
  }

  /// Conversion selects a named enum variant with an optional source.
  #[derive(Debug, Error)]
  #[error("...")]
  enum ErrorEnumOptional {
    /// Native source is wrapped by the selected variant.
    Test {
      /// Concrete optional conversion source.
      #[from]
      source: Option<IoError>,
    },
  }

  /// Distinct native conversion types select distinct variants.
  #[derive(Debug, Error)]
  #[error("...")]
  enum Many {
    /// Test-only anyhow interoperability preserves its underlying concrete error.
    Any(#[from] anyhow::Error),
    /// Standard I/O conversion retains its native source directly.
    Io(#[from] IoError),
  }

  /// Inspect source identity and kind while retaining the complete converted owner.
  ///
  /// # Errors
  /// Returns the original owner when its display, concrete source, or native kind differs.
  fn ensure_io_source<Subject: StdError>(subject: Subject, expected: ErrorKind) -> Result<Subject, PredicateFailure<Subject>> {
    ensure_that(subject, "conversion preserves the complete native I/O source", |observed| {
      observed.to_string() == "..."
        && observed
          .source()
          .and_then(|cause| cause.downcast_ref::<IoError>())
          .is_some_and(|cause| cause.kind() == expected)
    })
  }

  /// Each conversion shape executes its own assertion and retains its own concrete failure.
  macro_rules! conversion_cases {
    ($($name:ident => $target:ty),+ $(,)?) => {
      $(
        /// Derived From preserves this shape's native source and display.
        #[test]
        fn $name() -> Result<(), PredicateFailure<$target>> {
          ensure_io_source(<$target>::from(IoError::from(ErrorKind::NotFound)), ErrorKind::NotFound).map(drop)
        }
      )+
    };
  }

  conversion_cases! {
    test_from => ErrorStruct,
    optional_named_conversion_preserves_source => ErrorStructOptional,
    tuple_conversion_preserves_source => ErrorTuple,
    optional_tuple_conversion_preserves_source => ErrorTupleOptional,
    enum_conversion_preserves_source => ErrorEnum,
    optional_enum_conversion_preserves_source => ErrorEnumOptional,
  }

  /// The I/O conversion cannot select the anyhow variant.
  #[test]
  fn io_conversion_selects_its_native_variant() -> Result<(), PredicateFailure<Many>> {
    let converted = Many::from(IoError::from(ErrorKind::PermissionDenied));
    ensure_that(converted, "native I/O conversion selects its own variant", |observed| {
      matches!(*observed, Many::Io(ref cause) if cause.kind() == ErrorKind::PermissionDenied) && observed.to_string() == "..."
    })
    .map(drop)
  }

  /// Anyhow conversion preserves its original typed source and selects the distinct variant.
  #[test]
  fn anyhow_conversion_preserves_typed_source() -> Result<(), PredicateFailure<Many>> {
    let converted = Many::from(anyhow::Error::new(IoError::from(ErrorKind::Interrupted)));
    ensure_that(converted, "anyhow conversion retains the typed native I/O error", |observed| {
      matches!(*observed, Many::Any(ref cause) if cause.downcast_ref::<IoError>().is_some_and(|native| native.kind() == ErrorKind::Interrupted))
        && observed.to_string() == "..."
    }).map(drop)
  }

  /// Original named, numbered, and enum owners with absent optional sources.
  type OptionalOwners = (ErrorStructOptional, ErrorTupleOptional, ErrorEnumOptional);

  /// Optional sources remain absent when no conversion source was supplied.
  #[test]
  fn optional_sources_preserve_absence() -> Result<(), PredicateFailure<OptionalOwners>> {
    let owners = (
      ErrorStructOptional {
        source: None
      },
      ErrorTupleOptional(None),
      ErrorEnumOptional::Test {
        source: None
      },
    );
    ensure_that(owners, "optional wrappers do not invent missing sources", |observed| {
      observed.0.source().is_none() && observed.1.source().is_none() && observed.2.source().is_none()
    })
    .map(drop)
  }
}
