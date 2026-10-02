//! Display messages preserve field shorthand, expressions, escaping, and formatter paths.

/// Shared native display comparison vocabulary.
pub mod rendering;

/// Exercise the complete derive contract through native Rust values.
#[cfg(test)]
mod tests {
  use core::fmt::Display;
  use core::fmt::Formatter;
  use core::fmt::Octal;
  use core::fmt::Result as FormatResult;
  use std::error::Error as StdError;

  use strict_test_support::ensure_that;
  use thiserror::Error;

  use super::rendering::ensure_renderings;
  use super::rendering::ensure_renders;

  #[test]
  fn test_braced() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("braced error: {msg}")]
    struct Error {
      msg: String,
    }

    let msg = "T".to_owned();
    ensure_renders("braced error: T", Error {
      msg,
    })
    .map(drop)
  }

  #[test]
  fn test_braced_unused() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("braced error")]
    struct Error {
      extra: usize,
    }

    ensure_that(
      Error {
        extra: 7
      },
      "unformatted fields retain their original values without entering the message",
      |subject| subject.extra == 7 && subject.to_string() == "braced error",
    )
    .map(drop)
  }

  #[test]
  fn test_tuple() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("tuple error: {0}")]
    struct Error(usize);

    ensure_renders("tuple error: 0", Error(0)).map(drop)
  }

  #[test]
  fn test_unit() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("unit error")]
    struct Error;

    ensure_renders("unit error", Error).map(drop)
  }

  #[test]
  fn test_enum() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    enum Error {
      #[error("braced error: {id}")]
      Braced { id: usize },
      #[error("tuple error: {0}")]
      Tuple(usize),
      #[error("unit error")]
      Unit,
    }

    ensure_renderings(vec![
      ("braced error: 0", Error::Braced {
        id: 0
      }),
      ("tuple error: 0", Error::Tuple(0)),
      ("unit error", Error::Unit),
    ])
    .map(drop)
  }

  #[test]
  fn test_constants() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("{MSG}: {id:?} (code {CODE:?})")]
    struct Error {
      id: &'static str,
    }

    const MSG: &str = "failed to do";
    const CODE: usize = 9;

    ensure_renders("failed to do: \"\" (code 9)", Error {
      id: ""
    })
    .map(drop)
  }

  #[test]
  fn test_inherit() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("{0}")]
    enum Error {
      Some(&'static str),
      #[error("other error")]
      Other(&'static str),
    }

    ensure_that(
      (Error::Some("some error"), Error::Other("...")),
      "inherited formatting preserves the unused variant payload",
      |subject| matches!(subject.1, Error::Other("...")) && subject.0.to_string() == "some error" && subject.1.to_string() == "other error",
    )
    .map(drop)
  }

  #[test]
  fn test_brace_escape() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("fn main() {{}}")]
    struct Error;

    ensure_renders("fn main() {}", Error).map(drop)
  }

  #[test]
  fn test_expr() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("1 + 1 = {}", 1 + 1)]
    struct Error;
    ensure_renders("1 + 1 = 2", Error).map(drop)
  }

  #[test]
  fn test_nested() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("!bool = {}", not(*.0))]
    struct Error(bool);

    fn not(flag: bool) -> bool {
      !flag
    }

    ensure_renders("!bool = false", Error(true)).map(drop)
  }

  #[test]
  fn test_match() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("{intro}: {0}", intro = .1.as_ref().map_or_else(|| "there was an empty error".to_owned(), |number| format!("error occurred with {number}")))]
    struct Error(String, Option<usize>);

    ensure_renderings(vec![
      ("error occurred with 1: ...", Error("...".to_owned(), Some(1))),
      ("there was an empty error: ...", Error("...".to_owned(), None)),
    ])
    .map(drop)
  }

  #[test]
  fn test_nested_display() -> Result<(), impl StdError> {
    // Same behavior as the one in `test_match`, but without String allocations.
    #[derive(Error, Debug)]
    #[error("{}", {
          struct Msg<'a>(&'a str, &'a Option<usize>);
          impl Display for Msg<'_> {
              fn fmt(&self, formatter: &mut Formatter<'_>) -> FormatResult {
                  match *self.1 {
                      Some(n) => write!(formatter, "error occurred with {n}"),
                      None => formatter.write_str("there was an empty error"),
                  }?;
                  write!(formatter, ": {}", self.0)
              }
          }
          Msg(.0, .1)
      })]
    struct Error(String, Option<usize>);

    ensure_renderings(vec![
      ("error occurred with 1: ...", Error("...".to_owned(), Some(1))),
      ("there was an empty error: ...", Error("...".to_owned(), None)),
    ])
    .map(drop)
  }

  /// Full rustc integration checks an uninhabited error's generated standard contracts.
  #[test]
  fn test_void() -> Result<(), trybuild::TryBuildError> {
    let mut cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass/empty-enum.rs");
    cases.compile_fail("tests/ui/union.rs");
    cases.run()
  }

  #[test]
  fn test_mixed() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("a={first} :: b={} :: c={third} :: d={last}", 1, third = 2, last = 3)]
    struct Error {
      first: usize,
      last:  usize,
    }

    ensure_that(
      Error {
        first: 0, last: 0
      },
      "explicit arguments shadow fields while retaining the original native data",
      |subject| subject.first == 0 && subject.last == 0 && subject.to_string() == "a=0 :: b=1 :: c=2 :: d=3",
    )
    .map(drop)
  }

  #[test]
  fn test_ints() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    enum Error {
      #[error("error {0}")]
      Tuple(usize, usize),
      #[error("error {0}", '?')]
      Struct { marker: usize },
    }

    ensure_that(
      (Error::Tuple(9, 3), Error::Struct {
        marker: 4
      }),
      "numbered arguments leave non-interpolated native fields intact",
      |subject| {
        matches!(subject.0, Error::Tuple(9, 3))
          && matches!(subject.1, Error::Struct {
            marker: 4
          })
          && subject.0.to_string() == "error 9"
          && subject.1.to_string() == "error ?"
      },
    )
    .map(drop)
  }

  #[test]
  fn test_trailing_comma() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
      #[error(
          "error {0}",
      )]
      #[rustfmt::skip]
      struct Error(char);

    ensure_renders("error ?", Error('?')).map(drop)
  }

  #[test]
  fn test_field() -> Result<(), impl StdError> {
    #[derive(Debug)]
    struct Inner {
      data: usize,
    }

    #[derive(Error, Debug)]
    #[error("{}", .0.data)]
    struct Error(Inner);

    ensure_renders(
      "0",
      Error(Inner {
        data: 0
      }),
    )
    .map(drop)
  }

  #[test]
  fn test_nested_tuple_field() -> Result<(), impl StdError> {
    #[derive(Debug)]
    struct Inner(usize);

    #[derive(Error, Debug)]
    #[error("{}", .0.0)]
    struct Error(Inner);

    ensure_renders("0", Error(Inner(0))).map(drop)
  }

  #[test]
  fn test_pointer() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("{field:p}")]
    struct Struct {
      field: Box<i32>,
    }

    ensure_that(
      Struct {
        field: Box::new(-1)
      },
      "pointer shorthand retains the original owned allocation",
      |subject| subject.to_string() == format!("{:p}", subject.field),
    )
    .map(drop)
  }

  #[test]
  fn test_macro_rules_variant_from_call_site() -> Result<(), impl StdError> {
    // Regression test for https://github.com/dtolnay/thiserror/issues/86

    macro_rules! decl_error {
      ($variant:ident($value:ident)) => {
        #[derive(Error, Debug)]
        enum Error0 {
          #[error("{0:?}")]
          $variant($value),
        }

        #[derive(Error, Debug)]
        #[error("{0:?}")]
        enum Error1 {
          $variant($value),
        }
      };
    }

    decl_error!(Repro(u8));

    ensure_that(
      (Error0::Repro(0), Error1::Repro(0)),
      "macro call-site hygiene preserves both local message forms",
      |observed| observed.0.to_string() == "0" && observed.1.to_string() == "0",
    )
    .map(drop)
  }

  #[test]
  fn test_macro_rules_message_from_call_site() -> Result<(), impl StdError> {
    // Regression test for https://github.com/dtolnay/thiserror/issues/398

    macro_rules! decl_error {
          ($($errors:tt)*) => {
              #[derive(Error, Debug)]
              enum Error {
                  $($errors)*
              }
          };
      }

    decl_error! {
        #[error("{0}")]
        Unnamed(u8),
        #[error("{x}")]
        Named { x: u8 },
    }

    ensure_renderings(vec![
      ("0", Error::Unnamed(0)),
      ("0", Error::Named {
        x: 0
      }),
    ])
    .map(drop)
  }

  #[test]
  fn test_raw() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("braced raw error: {fn}")]
    struct Error {
      r#fn: &'static str,
    }

    ensure_renders("braced raw error: T", Error {
      r#fn: "T"
    })
    .map(drop)
  }

  #[test]
  fn test_raw_enum() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    enum Error {
      #[error("braced raw error: {fn}")]
      Braced { r#fn: &'static str },
    }

    ensure_renders("braced raw error: T", Error::Braced {
      r#fn: "T"
    })
    .map(drop)
  }

  #[test]
  fn test_keyword() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("error: {type}", type = 1)]
    struct Error;

    ensure_renders("error: 1", Error).map(drop)
  }

  #[test]
  fn test_self() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error("error: {self:?}")]
    struct Error;

    ensure_renders("error: Error", Error).map(drop)
  }

  #[test]
  fn test_str_special_chars() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    enum Error {
      #[error("brace left {{")]
      BraceLeft,
      #[error("brace left 2 \x7B\x7B")]
      BraceLeft2,
      #[error("brace left 3 \u{7B}\u{7B}")]
      BraceLeft3,
      #[error("brace right }}")]
      BraceRight,
      #[error("brace right 2 \x7D\x7D")]
      BraceRight2,
      #[error("brace right 3 \u{7D}\u{7D}")]
      BraceRight3,
      #[error("new_line")]
      NewLine,
      #[error("escape24 \u{78}")]
      Escape24,
    }

    ensure_renderings(vec![
      ("brace left {", Error::BraceLeft),
      ("brace left 2 {", Error::BraceLeft2),
      ("brace left 3 {", Error::BraceLeft3),
      ("brace right }", Error::BraceRight),
      ("brace right 2 }", Error::BraceRight2),
      ("brace right 3 }", Error::BraceRight3),
      ("new_line", Error::NewLine),
      ("escape24 x", Error::Escape24),
    ])
    .map(drop)
  }

  #[test]
  fn test_raw_str() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    enum Error {
      #[error(r#"raw "brace left {{""#)]
      BraceLeft,
      #[error(r"raw brace left 2 \x7B")]
      BraceLeft2,
      #[error(r#"raw "brace right }}""#)]
      BraceRight,
      #[error(r"raw brace right 2 \x7D")]
      BraceRight2,
    }

    ensure_renderings(vec![
      ("raw \"brace left {\"", Error::BraceLeft),
      (r"raw brace left 2 \x7B", Error::BraceLeft2),
      ("raw \"brace right }\"", Error::BraceRight),
      (r"raw brace right 2 \x7D", Error::BraceRight2),
    ])
    .map(drop)
  }

  mod util {
    use core::fmt::Formatter;
    use core::fmt::Octal;
    use core::fmt::Result as FormatResult;

    pub(super) fn octal<T: Octal>(number: &T, formatter: &mut Formatter<'_>) -> FormatResult {
      write!(formatter, "0o{number:o}")
    }
  }

  #[test]
  fn test_fmt_path() -> Result<(), impl StdError> {
    fn unit(formatter: &mut Formatter<'_>) -> FormatResult {
      formatter.write_str("unit=")
    }

    fn pair<Left: Display, Right: Display>(left: &Left, right: &Right, formatter: &mut Formatter<'_>) -> FormatResult {
      write!(formatter, "pair={left}:{right}")
    }

    #[derive(Error, Debug)]
    enum Error {
      #[error(fmt = unit)]
      Unit,
      #[error(fmt = pair)]
      Tuple(i32, i32),
      #[error(fmt = pair)]
      Entry { left: i32, right: i32 },
      #[error(fmt = util::octal)]
      I16(i16),
      #[error(fmt = util::octal::<i32>)]
      I32 { n: i32 },
      #[error(fmt = Octal::fmt)]
      I64(i64),
      #[error("...{0}")]
      Other(bool),
    }

    ensure_renderings(vec![
      ("unit=", Error::Unit),
      ("pair=10:0", Error::Tuple(10, 0)),
      ("pair=10:0", Error::Entry {
        left: 10, right: 0
      }),
      ("0o777", Error::I16(0o777)),
      ("0o777", Error::I32 {
        n: 0o777
      }),
      ("777", Error::I64(0o777)),
      ("...false", Error::Other(false)),
    ])
    .map(drop)
  }

  #[test]
  fn test_fmt_path_inherited() -> Result<(), impl StdError> {
    #[derive(Error, Debug)]
    #[error(fmt = util::octal)]
    enum Error {
      I16(i16),
      I32 {
        n: i32,
      },
      #[error(fmt = Octal::fmt)]
      I64(i64),
      #[error("...{0}")]
      Other(bool),
    }

    ensure_renderings(vec![
      ("0o777", Error::I16(0o777)),
      ("0o777", Error::I32 {
        n: 0o777
      }),
      ("777", Error::I64(0o777)),
      ("...false", Error::Other(false)),
    ])
    .map(drop)
  }
}
