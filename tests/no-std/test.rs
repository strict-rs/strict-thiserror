#![no_std]

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
  #[error("Error::E")]
  E(#[from] SourceError),
}

#[derive(Error, Debug)]
#[error("SourceError {field}")]
pub struct SourceError {
  pub field: i32,
}

#[cfg(test)]
mod tests {
  use core::error::Error as _;
  use core::fmt::Write;
  use core::fmt::{
    self,
  };
  use core::mem;
  use core::str;

  use strict_test_support::TestFailure;
  use strict_test_support::ensure_eq;
  use strict_test_support::ensure_ok;
  use strict_test_support::ensure_some;

  use crate::Error;
  use crate::SourceError;

  struct Buf<'a>(&'a mut [u8]);

  impl Write for Buf<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
      if s.len() <= self.0.len()
        && let Some((out, rest)) = mem::take(&mut self.0).split_at_mut_checked(s.len())
      {
        for (slot, byte) in out.iter_mut().zip(s.bytes()) {
          *slot = byte;
        }
        self.0 = rest;
        Ok(())
      } else {
        Err(fmt::Error)
      }
    }
  }

  #[test]
  fn test() -> Result<(), TestFailure> {
    let source = SourceError {
      field: -1
    };
    let error = Error::from(source);

    let source = ensure_some(error.source(), "the enum error exposes its #[from] source")?;
    let source = ensure_some(source.downcast_ref::<SourceError>(), "the exposed source downcasts to SourceError")?;

    let mut msg = [b'~'; 17];
    ensure_ok(write!(Buf(&mut msg), "{error}"), "the enum display renders into the fixed buffer")?;
    let rendered = ensure_ok(str::from_utf8(&msg), "the rendered enum message is UTF-8")?;
    ensure_eq(&rendered, &"Error::E~~~~~~~~~", "the enum display is preserved without std")?;

    let mut msg = [b'~'; 17];
    ensure_ok(
      write!(Buf(&mut msg), "{source}"),
      "the source display renders into the fixed buffer",
    )?;
    let rendered = ensure_ok(str::from_utf8(&msg), "the rendered source message is UTF-8")?;
    ensure_eq(&rendered, &"SourceError -1~~~", "the source display is preserved without std")
  }
}
