//! Display assertions retain their native owner and both compared messages.

use core::fmt::Display;
use std::vec::IntoIter;

use strict_test_support::ComparisonFailure;
use strict_test_support::ensure_eq;

/// Both owned messages accepted by one display comparison.
pub type RenderedMessages = (String, String);

/// A rejected display comparison retains the complete original display subject.
#[derive(Debug, thiserror::Error)]
#[error("display comparison for {subject} failed: {source}")]
pub struct RenderingFailure<Subject> {
  /// Original native subject whose display was checked.
  pub subject: Subject,
  /// Original native message comparison failure.
  #[source]
  pub source:  ComparisonFailure<String, String>,
}

/// Complete accepted subject and both native rendered messages.
pub type Rendered<Subject> = (Subject, RenderedMessages);

/// Compare a native displayable subject with its exact expected rendering.
///
/// # Errors
/// Returns the complete original subject and both messages when the comparison fails.
pub fn ensure_renders<Subject: Display>(expected: &str, actual: Subject) -> Result<Rendered<Subject>, RenderingFailure<Subject>> {
  match ensure_eq(
    actual.to_string(),
    expected.to_owned(),
    "derived Display retains the expected native message",
  ) {
    Ok(messages) => Ok((actual, messages)),
    Err(source) => Err(RenderingFailure {
      subject: actual,
      source,
    }),
  }
}

/// A failed display batch retains complete preceding owners and the unconsumed native cases.
#[derive(Debug, thiserror::Error)]
#[error("{failed}")]
pub struct RenderingBatchFailure<Subject> {
  /// Accepted native owners and their original compared messages.
  pub accepted:  Vec<Rendered<Subject>>,
  /// Complete original failing display owner and comparison.
  #[source]
  pub failed:    RenderingFailure<Subject>,
  /// Unconsumed native cases retained after the rejected comparison.
  pub remaining: IntoIter<(&'static str, Subject)>,
}

/// Complete native outcomes or a boxed failure retaining all batch owners and remaining cases.
pub type RenderingBatchResult<Subject> = Result<Vec<Rendered<Subject>>, Box<RenderingBatchFailure<Subject>>>;

/// Compare a sequence of native subjects without discarding accepted owners or remaining cases.
///
/// # Errors
/// Returns all accepted observations, the rejected native subject, and the original remainder.
pub fn ensure_renderings<Subject: Display>(cases: Vec<(&'static str, Subject)>) -> RenderingBatchResult<Subject> {
  let mut remaining = cases.into_iter();
  let mut accepted = Vec::new();
  while let Some((expected, subject)) = remaining.next() {
    match ensure_renders(expected, subject) {
      Ok(observation) => accepted.push(observation),
      Err(failed) => {
        return Err(Box::new(RenderingBatchFailure {
          accepted,
          failed,
          remaining,
        }));
      }
    }
  }
  Ok(accepted)
}
