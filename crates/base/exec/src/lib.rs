//! One way out of the process: a child, run to completion or on a pseudo-terminal.

mod error;
pub mod pty;
mod redact;
pub mod run;

pub use error::{Error, Result};
pub use redact::redact;
