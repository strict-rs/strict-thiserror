use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("I/O failed")]
struct Error(#[from] io::Error);

fn main() {
  let error = Error::from(io::Error::other("read failed"));
  let _rendered = error.to_string();
  let _source = std::error::Error::source(&error);
}
