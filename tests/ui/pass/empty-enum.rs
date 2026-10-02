use std::error::Error as StdError;
use thiserror::Error;

/// Uninhabited errors still receive valid standard trait implementations.
#[derive(Debug, Error)]
#[error("uninhabited error")]
pub enum UninhabitedError {}

/// Type checking proves the generated contract without inventing an error value.
pub fn borrow_error(subject: &UninhabitedError) -> &(dyn StdError + 'static) {
    subject
}

fn main() {}
