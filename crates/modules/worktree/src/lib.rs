//! Repos and worktrees: clones under `<root>/main`, worktrees under `<root>/worktrees/<session>/<project>/<branch>`.

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
use groove_types::SessionId;
pub use groove_types::{PoolEntry, WorktreeSpec};
pub use layout::Layout;

/// The module's handle: the database and the root, cheap to clone into a job.
#[derive(Clone)]
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

    /// The session's own directory, made when it is missing.
    pub fn session_dir(&self, session: &SessionId) -> Result<PathBuf> {
        let dir = self.layout.session_dir(session.as_str());
        std::fs::create_dir_all(&dir).map_err(|source| Error::Io {
            path: dir.clone(),
            source,
        })?;
        Ok(dir)
    }
}
