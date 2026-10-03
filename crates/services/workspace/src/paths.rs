//! The worktree's own files: one made, moved, copied or taken away.

use std::path::Path;

use groove_types::Result;

/// What one path operation does to the worktree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathOp {
    Create { path: String, folder: bool },
    Rename { from: String, to: String },
    Copy { from: String, to: String },
    Delete { path: String },
}

impl PathOp {
    /// A new file or folder named `name` inside `dir`, which may be the worktree root itself.
    pub fn made(dir: &str, name: &str, folder: bool) -> Self {
        let path = match dir.is_empty() {
            true => name.to_string(),
            false => format!("{dir}/{name}"),
        };
        PathOp::Create { path, folder }
    }

    /// `from` renamed `name`, in the same directory.
    pub fn renamed(from: &str, name: &str) -> Self {
        PathOp::Rename {
            from: from.to_string(),
            to: beside(from, name),
        }
    }

    /// A copy of `from` named `name`, in the same directory.
    pub fn copied(from: &str, name: &str) -> Self {
        PathOp::Copy {
            from: from.to_string(),
            to: beside(from, name),
        }
    }
}

/// A name in the same directory as the path it is given to.
fn beside(path: &str, name: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/{name}"),
        None => name.to_string(),
    }
}

/// One path operation, under the worktree and nowhere else.
pub fn path_op(dir: &Path, op: &PathOp) -> Result<()> {
    match op {
        PathOp::Create { path, folder } => groove_editor::create(dir, path, *folder),
        PathOp::Rename { from, to } => groove_editor::rename(dir, from, to),
        PathOp::Copy { from, to } => groove_editor::copy(dir, from, to),
        PathOp::Delete { path } => groove_editor::delete(dir, path),
    }
}
