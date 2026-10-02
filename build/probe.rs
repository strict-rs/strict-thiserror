//! Exercise the nightly error member-access API required by backtrace forwarding.

#![no_std]
#![feature(error_generic_member_access)]

use core::error::Error;
use core::error::Request;
use core::fmt;
use core::fmt::Debug;
use core::fmt::Display;

/// Source that registers its concrete payload in a member-access request.
struct MyError(Thing);
/// Payload requested from the probe error.
#[derive(Debug)]
struct Thing;

impl Debug for MyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("MyError").field(&self.0).finish()
    }
}

impl Display for MyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("member-access probe")
    }
}

impl Error for MyError {
    fn provide<'a>(&'a self, request: &mut Request<'a>) {
        request.provide_ref(&self.0);
    }
}

// Include in sccache cache key.
const _: Option<&str> = option_env!("RUSTC_BOOTSTRAP");
