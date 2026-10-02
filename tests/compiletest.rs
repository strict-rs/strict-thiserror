//! Full compiler integration preserves positive derives and native negative diagnostics.

/// Compiler diagnostics are compared through the strict typed runner.
#[cfg(test)]
mod tests {
  /// Every negative fixture and positive execution fixture participates in the same typed run.
  #[rustversion::attr(not(nightly), ignore = "requires nightly")]
  #[cfg_attr(miri, ignore = "incompatible with miri")]
  #[test]
  fn ui() -> Result<(), trybuild::TryBuildError> {
    let mut cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
    cases.pass("tests/ui/pass/*.rs");
    cases.run()
  }
}
