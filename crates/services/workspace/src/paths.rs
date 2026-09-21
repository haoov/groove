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

/// One path operation, under the worktree and nowhere else.
pub fn path_op(dir: &Path, op: &PathOp) -> Result<()> {
    match op {
        PathOp::Create { path, folder } => groove_editor::create(dir, path, *folder),
        PathOp::Rename { from, to } => groove_editor::rename(dir, from, to),
        PathOp::Copy { from, to } => groove_editor::copy(dir, from, to),
        PathOp::Delete { path } => groove_editor::delete(dir, path),
    }
}
