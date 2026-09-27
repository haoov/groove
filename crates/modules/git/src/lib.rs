//! Git, one repository at a time: one command through `exec::run`, one typed answer.

mod branches;
mod command;
mod diff;
mod error;
mod facts;
mod index;
mod log;
pub mod parse;
mod remote;
mod status;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use error::{Error, Result};
pub use parse::{Change, Counts, RemoteUrl};

/// A repository or worktree directory to run git in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Git {
    dir: PathBuf,
}

impl Git {
    pub fn at(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}
