//! One way out of the process: a child, run to completion, on a pseudo-terminal, or piped for as long as it lives.

/// The `args`, `cwd`, `env` and `describe` of a builder holding `program`, `args`, `cwd`, `env`.
macro_rules! described {
    () => {
        pub fn args<I, S>(mut self, args: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: Into<String>,
        {
            self.args.extend(args.into_iter().map(Into::into));
            self
        }

        pub fn cwd(mut self, dir: impl AsRef<std::path::Path>) -> Self {
            self.cwd = Some(dir.as_ref().to_path_buf());
            self
        }

        pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
            self.env.push((key.into(), value.into()));
            self
        }

        /// The command line with credentials redacted, for messages and logs.
        pub fn describe(&self) -> String {
            crate::redact::redact(&format!("{} {}", self.program, self.args.join(" ")))
        }
    };
}
pub(crate) use described;

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
