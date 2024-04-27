#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

#[cfg(feature = "arc")]
mod arc;
#[cfg(feature = "rc")]
mod rc;

#[cfg(feature = "arc")]
pub use arc::{empty_arc_array, empty_arc_slice, empty_arc_str};

#[cfg(feature = "rc")]
pub use rc::{empty_rc_array, empty_rc_slice, empty_rc_str};
