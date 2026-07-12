#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::panic)]
#![warn(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod as_poll_fd;
mod client;
mod error;
mod runtime;
mod server;
mod service;

pub use as_poll_fd::AsPollFd;
pub use error::Error;
pub use runtime::Runtime;
pub use service::Service;
