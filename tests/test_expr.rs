//! Caller expressions retain native control flow, formatting, and associated-type constraints.

/// Expressions are exercised with native typed error owners.
#[cfg(test)]
mod tests {
  use core::fmt::Display;
  #[cfg(feature = "std")]
  use std::path::PathBuf;

  use strict_test_support::PredicateFailure;
  use strict_test_support::ensure_that;
  use thiserror::Error;

  /// Native compiler diagnostics format their original control-flow inputs.
  #[derive(Debug, Error)]
  enum CompilerError {
    /// Shift direction and bounds remain native fields.
    #[error("cannot shift {} by {maximum} or more bits (got {current})", if *.is_left { "left" } else { "right" })]
    TooManyShiftBits {
      /// Native direction.
      is_left: bool,
      /// Original supported bound.
      maximum: u64,
      /// Original observed shift.
      current: u64,
    },
    /// User tokens are formatted without an intermediate cloned collection.
    #[error("#error {}", .0.join(" "))]
    User(Vec<&'static str>),
    /// Nested optional control flow preserves all sign states.
    #[error("overflow while parsing {}integer literal", match *.is_signed {
      Some(true) => "signed ",
      Some(false) => "unsigned ",
      None => "",
    })]
    IntegerOverflow {
      /// Original optional sign.
      is_signed: Option<bool>,
    },
    /// An if-let expression remains independent of match-based formatting.
    #[error("overflow while parsing {}integer literal", .is_signed.as_ref().map_or("", |signed| if *signed { "signed " } else { "unsigned " }))]
    IntegerOverflowConditional {
      /// Original optional sign.
      is_signed: Option<bool>,
    },
  }

  /// Caller expressions retain both polarities of each native control-flow input.
  #[test]
  fn test_rcc() -> Result<(), PredicateFailure<Vec<CompilerError>>> {
    let owners = vec![
      CompilerError::TooManyShiftBits {
        is_left: true,
        maximum: 32,
        current: 50,
      },
      CompilerError::TooManyShiftBits {
        is_left: false,
        maximum: 32,
        current: 50,
      },
      CompilerError::User(vec!["A", "B", "C"]),
      CompilerError::IntegerOverflow {
        is_signed: Some(true)
      },
      CompilerError::IntegerOverflow {
        is_signed: Some(false)
      },
      CompilerError::IntegerOverflow {
        is_signed: None
      },
      CompilerError::IntegerOverflowConditional {
        is_signed: Some(true)
      },
      CompilerError::IntegerOverflowConditional {
        is_signed: Some(false)
      },
      CompilerError::IntegerOverflowConditional {
        is_signed: None
      },
    ];
    ensure_that(
      owners,
      "native caller expressions preserve every supported control-flow input",
      |observed| {
        observed.iter().map(ToString::to_string).eq([
          "cannot shift left by 32 or more bits (got 50)",
          "cannot shift right by 32 or more bits (got 50)",
          "#error A B C",
          "overflow while parsing signed integer literal",
          "overflow while parsing unsigned integer literal",
          "overflow while parsing integer literal",
          "overflow while parsing signed integer literal",
          "overflow while parsing unsigned integer literal",
          "overflow while parsing integer literal",
        ])
      },
    )
    .map(drop)
  }

  /// Optional suggestions preserve the caller's original display contract.
  #[derive(Debug, Error)]
  enum RustupError {
    /// Native component lookup inputs and optional suggestion.
    #[error("toolchain '{name}' does not contain component {component}{}", .suggestion.as_ref().map_or_else(String::new, |suggested| format!("; did you mean '{suggested}'?")))]
    UnknownComponent {
      /// Original toolchain name.
      name:       String,
      /// Original component name.
      component:  String,
      /// Native optional suggestion.
      suggestion: Option<String>,
    },
  }

  /// Optional expression arguments preserve suggestion presence and absence.
  #[test]
  fn test_rustup() -> Result<(), PredicateFailure<Vec<RustupError>>> {
    let owners = vec![
      RustupError::UnknownComponent {
        name:       "nightly".to_owned(),
        component:  "clipy".to_owned(),
        suggestion: Some("clippy".to_owned()),
      },
      RustupError::UnknownComponent {
        name:       "nightly".to_owned(),
        component:  "clipy".to_owned(),
        suggestion: None,
      },
    ];
    ensure_that(
      owners,
      "native suggestion expressions preserve both presence and absence",
      |observed| {
        observed.iter().map(ToString::to_string).eq([
          "toolchain 'nightly' does not contain component clipy; did you mean 'clippy'?",
          "toolchain 'nightly' does not contain component clipy",
        ])
      },
    )
    .map(drop)
  }

  /// An associated type remains part of a real formatter argument constraint.
  #[cfg(feature = "std")]
  trait Constraint<Parameter>: Display {
    /// Complete associated native output type.
    type Output;
  }

  #[cfg(feature = "std")]
  impl<Parameter> Constraint<Parameter> for i32 {
    type Output = Self;
  }

  /// Keep the typed associated constraint at the caller boundary.
  #[cfg(feature = "std")]
  #[allow(
    clippy::single_call_fn,
    reason = "the associated-type equality is a native formatter-argument boundary rather than a field display requirement"
  )]
  fn constrained_display(subject: &impl Constraint<i32, Output = i32>) -> &impl Display {
    subject
  }

  /// Associated constraints remain parsed as expression arguments instead of field shorthand.
  #[cfg(feature = "std")]
  #[derive(Debug, Error)]
  #[error("{path} {number}", number = constrained_display(&0))]
  struct AssociatedArgument {
    /// Original native path view.
    path: PathBuf,
  }

  /// The formatter expression retains its concrete associated-type equality.
  #[cfg(feature = "std")]
  #[test]
  fn test_assoc_type_equality_constraint() -> Result<(), PredicateFailure<AssociatedArgument>> {
    ensure_that(
      AssociatedArgument {
        path: PathBuf::from("..."),
      },
      "associated constraints stay attached to the caller's native argument",
      |observed| observed.to_string() == "... 0",
    )
    .map(drop)
  }
}
