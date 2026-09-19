//! Repos and worktrees on disk and in the database. The pool of clones lives under
//! `<root>/main`, a session's worktrees under `<root>/worktrees/<session>/<project>/<branch>`.

mod error;
mod layout;
pub mod naming;
mod pool;
mod provision;
mod status;
mod store;
mod teardown;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use error::{Error, Result};
use groove_db::Db;
pub use groove_types::{PoolEntry, WorktreeSpec};
pub use layout::Layout;

/// The module's handle: the database and the root, cheap to clone into a job.
#[derive(std::clone::Clone)]
pub struct Pool {
    db: Db,
    layout: Layout,
}

impl Pool {
    pub fn new(db: Db, root: impl AsRef<Path>) -> Self {
        Self {
            db,
            layout: Layout::new(root.as_ref().to_path_buf()),
        }
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn root(&self) -> &PathBuf {
        &self.layout.root
    }
}
