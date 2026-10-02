#![deny(deprecated)]

use thiserror::Error;

/// A deprecated public error still receives its generated implementations.
#[deprecated]
#[derive(Debug, Error)]
#[error("deprecated struct")]
pub struct DeprecatedStruct;

/// A field's deprecation does not add a generated-code warning.
#[derive(Debug, Error)]
#[error("{message} {}", .message)]
pub struct DeprecatedField {
    #[deprecated]
    message: String,
}

/// A deprecated enum still receives its generated implementations.
#[deprecated]
#[derive(Debug, Error)]
pub enum DeprecatedEnum {
    #[error("deprecated enum")]
    Variant,
}

/// A deprecated variant can be handled by the generated display implementation.
#[derive(Debug, Error)]
pub enum DeprecatedVariant {
    #[deprecated]
    #[error("deprecated variant")]
    Variant,
}

fn main() {}
