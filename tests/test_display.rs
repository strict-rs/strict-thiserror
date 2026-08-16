#![allow(
  clippy::elidable_lifetime_names,
  clippy::needless_lifetimes,
  clippy::needless_raw_string_hashes,
  clippy::trivially_copy_pass_by_ref,
  clippy::uninlined_format_args
)]

use core::fmt::Display;
use core::fmt::{
  self,
};

use strict_test_support::TestFailure;
use strict_test_support::ensure_eq;
use thiserror::Error;

fn ensure_renders<T: Display>(expected: &str, actual: T) -> Result<(), TestFailure> {
  ensure_eq(
    &actual.to_string(),
    &expected.to_owned(),
    "the derived Display output matches the expected rendering",
  )
}

#[test]
fn test_braced() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("braced error: {msg}")]
  struct Error {
    msg: String,
  }

  let msg = "T".to_owned();
  ensure_renders("braced error: T", Error {
    msg,
  })
}

#[test]
fn test_braced_unused() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("braced error")]
  struct Error {
    extra: usize,
  }

  ensure_renders("braced error", Error {
    extra: 0
  })
}

#[test]
fn test_tuple() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("tuple error: {0}")]
  struct Error(usize);

  ensure_renders("tuple error: 0", Error(0))
}

#[test]
fn test_unit() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("unit error")]
  struct Error;

  ensure_renders("unit error", Error)
}

#[test]
fn test_enum() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  enum Error {
    #[error("braced error: {id}")]
    Braced { id: usize },
    #[error("tuple error: {0}")]
    Tuple(usize),
    #[error("unit error")]
    Unit,
  }

  ensure_renders("braced error: 0", Error::Braced {
    id: 0
  })?;
  ensure_renders("tuple error: 0", Error::Tuple(0))?;
  ensure_renders("unit error", Error::Unit)
}

#[test]
fn test_constants() -> Result<(), TestFailure> {
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
}

#[test]
fn test_inherit() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("{0}")]
  enum Error {
    Some(&'static str),
    #[error("other error")]
    Other(&'static str),
  }

  ensure_renders("some error", Error::Some("some error"))?;
  ensure_renders("other error", Error::Other("..."))
}

#[test]
fn test_brace_escape() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("fn main() {{}}")]
  struct Error;

  ensure_renders("fn main() {}", Error)
}

#[test]
fn test_expr() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("1 + 1 = {}", 1 + 1)]
  struct Error;
  ensure_renders("1 + 1 = 2", Error)
}

#[test]
fn test_nested() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("!bool = {}", not(.0))]
  struct Error(bool);

  #[allow(clippy::trivially_copy_pass_by_ref)]
  fn not(bool: &bool) -> bool {
    !*bool
  }

  ensure_renders("!bool = false", Error(true))
}

#[test]
fn test_match() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("{intro}: {0}", intro = match .1 {
        Some(n) => format!("error occurred with {}", n),
        None => "there was an empty error".to_owned(),
    })]
  struct Error(String, Option<usize>);

  ensure_renders("error occurred with 1: ...", Error("...".to_owned(), Some(1)))?;
  ensure_renders("there was an empty error: ...", Error("...".to_owned(), None))
}

#[test]
fn test_nested_display() -> Result<(), TestFailure> {
  // Same behavior as the one in `test_match`, but without String allocations.
  #[derive(Error, Debug)]
  #[error("{}", {
        struct Msg<'a>(&'a String, &'a Option<usize>);
        impl<'a> Display for Msg<'a> {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                match self.1 {
                    Some(n) => write!(formatter, "error occurred with {}", n),
                    None => write!(formatter, "there was an empty error"),
                }?;
                write!(formatter, ": {}", self.0)
            }
        }
        Msg(.0, .1)
    })]
  struct Error(String, Option<usize>);

  ensure_renders("error occurred with 1: ...", Error("...".to_owned(), Some(1)))?;
  ensure_renders("there was an empty error: ...", Error("...".to_owned(), None))
}

#[test]
fn test_void() {
  #[allow(clippy::empty_enums)]
  #[derive(Error, Debug)]
  #[error("...")]
  pub enum Error {}

  let _: Error;
}

#[test]
fn test_mixed() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("a={a} :: b={} :: c={c} :: d={d}", 1, c = 2, d = 3)]
  struct Error {
    a: usize,
    d: usize,
  }

  ensure_renders("a=0 :: b=1 :: c=2 :: d=3", Error {
    a: 0, d: 0
  })
}

#[test]
fn test_ints() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  enum Error {
    #[error("error {0}")]
    Tuple(usize, usize),
    #[error("error {0}", '?')]
    Struct { v: usize },
  }

  ensure_renders("error 9", Error::Tuple(9, 0))?;
  ensure_renders("error ?", Error::Struct {
    v: 0
  })
}

#[test]
fn test_trailing_comma() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
    #[error(
        "error {0}",
    )]
    #[rustfmt::skip]
    struct Error(char);

  ensure_renders("error ?", Error('?'))
}

#[test]
fn test_field() -> Result<(), TestFailure> {
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
}

#[test]
fn test_nested_tuple_field() -> Result<(), TestFailure> {
  #[derive(Debug)]
  struct Inner(usize);

  #[derive(Error, Debug)]
  #[error("{}", .0.0)]
  struct Error(Inner);

  ensure_renders("0", Error(Inner(0)))
}

#[test]
fn test_pointer() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("{field:p}")]
  pub struct Struct {
    field: Box<i32>,
  }

  let s = Struct {
    field: Box::new(-1)
  };
  ensure_eq(
    &s.to_string(),
    &format!("{:p}", s.field),
    "the {field:p} shorthand renders the box's pointer address",
  )
}

#[test]
fn test_macro_rules_variant_from_call_site() -> Result<(), TestFailure> {
  // Regression test for https://github.com/dtolnay/thiserror/issues/86

  macro_rules! decl_error {
    ($variant:ident($value:ident)) => {
      #[derive(Error, Debug)]
      pub enum Error0 {
        #[error("{0:?}")]
        $variant($value),
      }

      #[derive(Error, Debug)]
      #[error("{0:?}")]
      pub enum Error1 {
        $variant($value),
      }
    };
  }

  decl_error!(Repro(u8));

  ensure_renders("0", Error0::Repro(0))?;
  ensure_renders("0", Error1::Repro(0))
}

#[test]
fn test_macro_rules_message_from_call_site() -> Result<(), TestFailure> {
  // Regression test for https://github.com/dtolnay/thiserror/issues/398

  macro_rules! decl_error {
        ($($errors:tt)*) => {
            #[derive(Error, Debug)]
            pub enum Error {
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

  ensure_renders("0", Error::Unnamed(0))?;
  ensure_renders("0", Error::Named {
    x: 0
  })
}

#[test]
fn test_raw() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("braced raw error: {fn}")]
  struct Error {
    r#fn: &'static str,
  }

  ensure_renders("braced raw error: T", Error {
    r#fn: "T"
  })
}

#[test]
fn test_raw_enum() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  enum Error {
    #[error("braced raw error: {fn}")]
    Braced { r#fn: &'static str },
  }

  ensure_renders("braced raw error: T", Error::Braced {
    r#fn: "T"
  })
}

#[test]
fn test_keyword() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("error: {type}", type = 1)]
  struct Error;

  ensure_renders("error: 1", Error)
}

#[test]
fn test_self() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error("error: {self:?}")]
  struct Error;

  ensure_renders("error: Error", Error)
}

#[test]
fn test_str_special_chars() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  pub enum Error {
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

  ensure_renders("brace left {", Error::BraceLeft)?;
  ensure_renders("brace left 2 {", Error::BraceLeft2)?;
  ensure_renders("brace left 3 {", Error::BraceLeft3)?;
  ensure_renders("brace right }", Error::BraceRight)?;
  ensure_renders("brace right 2 }", Error::BraceRight2)?;
  ensure_renders("brace right 3 }", Error::BraceRight3)?;
  ensure_renders("new_line", Error::NewLine)?;
  ensure_renders("escape24 x", Error::Escape24)
}

#[test]
fn test_raw_str() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  pub enum Error {
    #[error(r#"raw brace left {{"#)]
    BraceLeft,
    #[error(r#"raw brace left 2 \x7B"#)]
    BraceLeft2,
    #[error(r#"raw brace right }}"#)]
    BraceRight,
    #[error(r#"raw brace right 2 \x7D"#)]
    BraceRight2,
  }

  ensure_renders(r#"raw brace left {"#, Error::BraceLeft)?;
  ensure_renders(r#"raw brace left 2 \x7B"#, Error::BraceLeft2)?;
  ensure_renders(r#"raw brace right }"#, Error::BraceRight)?;
  ensure_renders(r#"raw brace right 2 \x7D"#, Error::BraceRight2)
}

mod util {
  use core::fmt::Octal;
  use core::fmt::{
    self,
  };

  pub fn octal<T: Octal>(number: &T, formatter: &mut fmt::Formatter) -> fmt::Result {
    write!(formatter, "0o{:o}", number)
  }
}

#[test]
fn test_fmt_path() -> Result<(), TestFailure> {
  fn unit(formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("unit=")
  }

  fn pair(k: &i32, v: &i32, formatter: &mut fmt::Formatter) -> fmt::Result {
    write!(formatter, "pair={k}:{v}")
  }

  #[derive(Error, Debug)]
  pub enum Error {
    #[error(fmt = unit)]
    Unit,
    #[error(fmt = pair)]
    Tuple(i32, i32),
    #[error(fmt = pair)]
    Entry { k: i32, v: i32 },
    #[error(fmt = crate::util::octal)]
    I16(i16),
    #[error(fmt = crate::util::octal::<i32>)]
    I32 { n: i32 },
    #[error(fmt = core::fmt::Octal::fmt)]
    I64(i64),
    #[error("...{0}")]
    Other(bool),
  }

  ensure_renders("unit=", Error::Unit)?;
  ensure_renders("pair=10:0", Error::Tuple(10, 0))?;
  ensure_renders("pair=10:0", Error::Entry {
    k: 10, v: 0
  })?;
  ensure_renders("0o777", Error::I16(0o777))?;
  ensure_renders("0o777", Error::I32 {
    n: 0o777
  })?;
  ensure_renders("777", Error::I64(0o777))?;
  ensure_renders("...false", Error::Other(false))
}

#[test]
fn test_fmt_path_inherited() -> Result<(), TestFailure> {
  #[derive(Error, Debug)]
  #[error(fmt = crate::util::octal)]
  pub enum Error {
    I16(i16),
    I32 {
      n: i32,
    },
    #[error(fmt = core::fmt::Octal::fmt)]
    I64(i64),
    #[error("...{0}")]
    Other(bool),
  }

  ensure_renders("0o777", Error::I16(0o777))?;
  ensure_renders("0o777", Error::I32 {
    n: 0o777
  })?;
  ensure_renders("777", Error::I64(0o777))?;
  ensure_renders("...false", Error::Other(false))
}
