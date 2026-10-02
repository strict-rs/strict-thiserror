use core::fmt;
use core::fmt::Pointer;

/// Borrow a pointer field while preserving the pointer's formatting implementation.
#[derive(Debug, Copy, Clone)]
pub struct Var<'a, T: ?Sized>(pub &'a T);

impl<T: Pointer + ?Sized> Pointer for Var<'_, T> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    Pointer::fmt(self.0, formatter)
  }
}
