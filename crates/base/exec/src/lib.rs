//! One way out of the process: a child, run to completion, on a pseudo-terminal, or piped for as long as it lives.

mod error;
pub mod login;
pub mod piped;
pub mod pty;
mod redact;
pub mod run;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use redact::redact;
