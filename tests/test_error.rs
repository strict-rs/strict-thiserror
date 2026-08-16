#![allow(dead_code)]

use core::fmt::Display;
use core::fmt::{
  self,
};
use std::io;

use thiserror::Error;

// The derive requires a `Display` impl to satisfy the `Error` supertrait, but
// these compile-shape fixtures are never formatted, so the impl just renders
// the type's own name.
macro_rules! type_name_display {
  ($ty:ty) => {
    impl Display for $ty {
      fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(stringify!($ty))
      }
    }
  };
}

#[derive(Error, Debug)]
struct BracedError {
  msg: String,
  pos: usize,
}

#[derive(Error, Debug)]
struct TupleError(String, usize);

#[derive(Error, Debug)]
struct UnitError;

#[derive(Error, Debug)]
struct WithSource {
  #[source]
  cause: io::Error,
}

#[derive(Error, Debug)]
struct WithAnyhow {
  #[source]
  cause: anyhow::Error,
}

#[derive(Error, Debug)]
enum EnumError {
  Braced {
    #[source]
    cause: io::Error,
  },
  Tuple(#[source] io::Error),
  Unit,
}

type_name_display!(BracedError);
type_name_display!(TupleError);
type_name_display!(UnitError);
type_name_display!(WithSource);
type_name_display!(WithAnyhow);
type_name_display!(EnumError);
