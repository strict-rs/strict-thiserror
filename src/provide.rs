use core::error::Error;
use core::error::Request;

#[doc(hidden)]
/// Forward a source's nightly generic member-access request through generated implementations.
pub trait ThiserrorProvide: Sealed {
  /// Let the source register its values in the caller's request.
  fn thiserror_provide<'a>(&'a self, request: &mut Request<'a>);
}

impl<T> ThiserrorProvide for T
where
  T: Error + ?Sized,
{
  #[inline]
  fn thiserror_provide<'a>(&'a self, request: &mut Request<'a>) {
    self.provide(request);
  }
}

#[doc(hidden)]
/// Restrict request forwarding to standard error subjects.
pub trait Sealed {}
impl<T: Error + ?Sized> Sealed for T {}
