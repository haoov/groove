//! One way out of the process: a child, run to completion or on a pseudo-terminal.

mod error;
pub mod login;
pub mod pty;
mod redact;
pub mod run;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use redact::redact;
