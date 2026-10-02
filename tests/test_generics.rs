//! Generic errors retain native field-specific bounds, sources, and associated-type failures.

/// Positive and negative formatting evidence uses the complete original generic owners.
#[cfg(test)]
mod tests {
  use core::fmt::Debug;
  use core::fmt::Display;
  use core::fmt::Formatter;
  use core::fmt::Result as FormatResult;
  use core::num::ParseIntError;
  use core::str::FromStr;
  use std::error::Error as _;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// This payload intentionally supports neither formatting trait.
  struct NoFormat;

  /// This payload supports Debug while intentionally lacking Display.
  #[derive(Debug)]
  struct DebugOnly;

  /// This payload supports Display while intentionally lacking Debug.
  struct DisplayOnly;

  impl Display for DisplayOnly {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
      formatter.write_str("display only")
    }
  }

  /// Generic debug formatting bounds the field through its complete native type.
  #[derive(Debug, Error)]
  enum EnumDebugGeneric<Source> {
    /// Native generic diagnostic payload.
    #[error("{0:?}")]
    FatalError(Source),
  }

  /// Conversion retains a complete generic error as its source.
  #[derive(Debug, Error)]
  enum EnumFromGeneric<Source> {
    /// Native generic conversion source.
    #[error("enum from generic")]
    Source(#[from] EnumDebugGeneric<Source>),
  }

  /// Distinct fields require only their actually used formatting traits.
  #[derive(Error)]
  enum EnumCompound<HasDisplay, HasDebug, HasNeither> {
    /// Both field-specific traits are used.
    #[error("{0} {1:?}")]
    DisplayDebug(HasDisplay, HasDebug),
    /// The second field deliberately has no formatting requirements.
    #[error("{0}")]
    Display(HasDisplay, HasNeither),
    /// The first field deliberately has no formatting requirements.
    #[error("{1:?}")]
    Debug(HasNeither, HasDebug),
  }

  impl<HasDisplay, HasDebug, HasNeither> Debug for EnumCompound<HasDisplay, HasDebug, HasNeither> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
      formatter.write_str("EnumCompound")
    }
  }

  /// Complete native variants exercise independent field-bound inference.
  type CompoundOwners = [EnumCompound<DisplayOnly, DebugOnly, NoFormat>; 3];

  /// Each variant retains only the formatting requirements its message uses.
  #[test]
  fn test_display_enum_compound() -> Result<(), PredicateFailure<CompoundOwners>> {
    let owners = [
      EnumCompound::DisplayDebug(DisplayOnly, DebugOnly),
      EnumCompound::Display(DisplayOnly, NoFormat),
      EnumCompound::Debug(NoFormat, DebugOnly),
    ];
    ensure_that(owners, "unused fields acquire neither Debug nor Display bounds", |observed| {
      observed
        .iter()
        .zip(["display only DebugOnly", "display only", "DebugOnly"])
        .all(|(subject, expected)| subject.to_string() == expected && subject.source().is_none())
    })
    .map(drop)
  }

  /// Transparent enum preserves a generic representation without inventing a source.
  #[derive(Debug, Error)]
  enum EnumTransparentGeneric<Source> {
    /// Complete native transparent owner.
    #[error(transparent)]
    Other(Source),
  }

  /// Generic struct formatting infers Debug on its original payload type.
  #[derive(Debug, Error)]
  #[error("{underlying:?}")]
  struct StructDebugGeneric<Source> {
    /// Complete native generic payload.
    underlying: Source,
  }

  /// Generic conversion leaves display ownership at the caller.
  #[derive(Debug, Error)]
  struct StructFromGeneric<Source> {
    /// Complete original generic source.
    #[from]
    source: StructDebugGeneric<Source>,
  }

  impl<Source> Display for StructFromGeneric<Source> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
      formatter.write_str("struct from generic")
    }
  }

  /// Transparent struct preserves the complete generic representation.
  #[derive(Debug, Error)]
  #[error(transparent)]
  struct StructTransparentGeneric<Source>(Source);

  /// The source's associated error type owns its display requirement.
  #[derive(Debug, Error)]
  enum AssociatedTypeError<Parser: FromStr> {
    /// Independent source-free variant.
    #[error("couldn't parse matrix")]
    Other,
    /// Concrete associated parse failure.
    #[error("couldn't parse entry: {0}")]
    EntryParseError(Parser::Err),
  }

  /// Original named and numbered generic conversion source owners.
  type ConversionOwners = (EnumFromGeneric<DebugOnly>, StructFromGeneric<DebugOnly>);

  /// Named and numbered generic wrappers retain their complete concrete source chain.
  #[test]
  fn generic_conversions_preserve_complete_sources() -> Result<(), PredicateFailure<ConversionOwners>> {
    let owners = (
      EnumFromGeneric::from(EnumDebugGeneric::FatalError(DebugOnly)),
      StructFromGeneric::from(StructDebugGeneric {
        underlying: DebugOnly
      }),
    );
    ensure_that(owners, "generic conversion bounds retain the concrete source types", |observed| {
      observed.0.to_string() == "enum from generic"
        && observed
          .0
          .source()
          .and_then(|cause| cause.downcast_ref::<EnumDebugGeneric<DebugOnly>>())
          .is_some_and(|cause| cause.to_string() == "DebugOnly")
        && observed.1.to_string() == "struct from generic"
        && observed
          .1
          .source()
          .and_then(|cause| cause.downcast_ref::<StructDebugGeneric<DebugOnly>>())
          .is_some_and(|cause| cause.to_string() == "DebugOnly")
    })
    .map(drop)
  }

  /// Original enum and struct representations of a source-free generic error.
  type TransparentOwners = (
    EnumTransparentGeneric<StructDebugGeneric<DebugOnly>>,
    StructTransparentGeneric<StructDebugGeneric<DebugOnly>>,
  );

  /// Transparent generic struct and enum owners retain source-free representations.
  #[test]
  fn generic_transparent_owners_preserve_absent_source() -> Result<(), PredicateFailure<TransparentOwners>> {
    let owners = (
      EnumTransparentGeneric::Other(StructDebugGeneric {
        underlying: DebugOnly
      }),
      StructTransparentGeneric(StructDebugGeneric {
        underlying: DebugOnly
      }),
    );
    ensure_that(
      owners,
      "generic transparent owners preserve both display and source absence",
      |observed| {
        observed.0.to_string() == "DebugOnly"
          && observed.0.source().is_none()
          && observed.1.to_string() == "DebugOnly"
          && observed.1.source().is_none()
      },
    )
    .map(drop)
  }

  /// Associated errors remain displayable without changing the original parse failure.
  #[test]
  fn associated_type_error_retains_its_native_parser_failure() -> Result<(), PredicateFailure<Result<i32, ParseIntError>>> {
    let parsed = "invalid".parse::<i32>();
    ensure_that(parsed, "associated parse errors retain their concrete native failure", |observed| {
      let Err(ref failure) = *observed else {
        return false;
      };
      let independent = AssociatedTypeError::<i32>::Other;
      let associated = AssociatedTypeError::<i32>::EntryParseError(failure.clone());
      independent.to_string() == "couldn't parse matrix"
        && independent.source().is_none()
        && associated.to_string() == format!("couldn't parse entry: {failure}")
        && associated.source().is_none()
    })
    .map(drop)
  }

  /// Explicit named arguments do not impose a field's unused display requirement.
  #[derive(Debug, Error)]
  #[error("{payload}", payload = "...")]
  struct NamedArgument<Payload> {
    /// Native field intentionally supports Debug only.
    payload: Payload,
  }

  /// An explicit named argument shadows the field without requiring Display.
  #[test]
  fn test_no_bound_on_named_fmt() -> Result<(), PredicateFailure<NamedArgument<DebugOnly>>> {
    ensure_that(
      NamedArgument {
        payload: DebugOnly
      },
      "explicit named formatting does not bound the unused field",
      |observed| observed.to_string() == "..." && observed.source().is_none(),
    )
    .map(drop)
  }

  /// Both native hexadecimal traits are required by one generic field.
  #[derive(Debug, Error)]
  #[error("0x{number:x} 0x{number:X}")]
  struct MultipleBounds<Number> {
    /// Original native numeric payload.
    number: Number,
  }

  /// Repeated field formatting retains both independently required native traits.
  #[test]
  fn test_multiple_bound() -> Result<(), PredicateFailure<MultipleBounds<i32>>> {
    ensure_that(
      MultipleBounds {
        number: 0xFF_i32
      },
      "the original field supports both hexadecimal formatting traits",
      |observed| observed.to_string() == "0xff 0xFF" && observed.source().is_none(),
    )
    .map(drop)
  }
}
