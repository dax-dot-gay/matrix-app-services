#![warn(missing_docs)]

//! Wrapper for the matrix_sdk crate that implements the AppService API.

///
pub mod types;
pub use types::{Config, Namespace};

///
mod error;
pub use error::Error;
pub(crate) use error::Result;

///
pub mod client;

///
pub mod servers;
