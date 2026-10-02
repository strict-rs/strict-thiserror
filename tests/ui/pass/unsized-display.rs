use std::fmt;
use thiserror::Error;

/// A trailing unsized field remains valid for generated Display and Error implementations.
#[derive(Debug, Error)]
#[error("{tail}")]
pub struct UnsizedFailure {
    pub head: i32,
    pub tail: str,
}

/// Rust type checking verifies the unsized owner's generated formatter without requiring a value.
pub fn format_unsized(subject: &UnsizedFailure, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Display::fmt(subject, formatter)
}

fn main() {}
