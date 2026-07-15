use thiserror::Error;

#[derive(Debug, Error)]
#[error("tuple fields: {0}, {1}")]
struct TupleFields(u8, u8);

#[derive(Debug, Error)]
#[error("named field: {0}", .value)]
struct NamedField {
  value: u8,
}

fn main() {
  let _tuple_rendered = TupleFields(1, 2).to_string();
  let _named_rendered = NamedField {
    value: 3
  }
  .to_string();
}
