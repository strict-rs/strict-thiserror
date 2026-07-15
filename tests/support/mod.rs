use std::fmt::Display;

use strict_test_support::TestFailure;
use strict_test_support::ensure_eq;
use strict_test_support::ensure_some;

pub fn ensure_display<T: Display + ?Sized>(value: &T, expected: &str, context: &'static str) -> Result<(), TestFailure> {
  let rendered = value.to_string();
  let expected = expected.to_owned();
  ensure_eq(&rendered, &expected, context)
}

pub fn ensure_source<'error, Source>(
  error: &'error (dyn std::error::Error + 'static),
  context: &'static str,
) -> Result<&'error Source, TestFailure>
where
  Source: std::error::Error + 'static,
{
  let source = ensure_some(error.source(), context)?;
  ensure_some(source.downcast_ref::<Source>(), context)
}
