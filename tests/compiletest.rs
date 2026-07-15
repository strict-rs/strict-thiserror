#[rustversion::attr(not(nightly), ignore = "requires nightly")]
#[cfg_attr(miri, ignore = "incompatible with miri")]
#[test]
fn ui() -> Result<(), trybuild::TryBuildError> {
  let mut cases = trybuild::TestCases::new();
  cases.compile_fail("tests/ui/*.rs");
  cases.pass("tests/ui/pass/*.rs");
  cases.run()
}
