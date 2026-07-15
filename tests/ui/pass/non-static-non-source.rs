use thiserror::Error;

#[derive(Debug, Error)]
#[error("borrowed input: {input}")]
struct BorrowedInput<'input> {
  input: &'input str,
}

fn main() {
  let _rendered = BorrowedInput {
    input: "token"
  }
  .to_string();
}
