use core::fmt::Pointer;
use core::fmt::{
  self,
};

pub struct Var<'a, T: ?Sized>(pub &'a T);

impl<'a, T: Pointer + ?Sized> Pointer for Var<'a, T> {
  fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    Pointer::fmt(self.0, formatter)
  }
}
