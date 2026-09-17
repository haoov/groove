//! What the editing surface needs outside the buffer: the files of one worktree, and
//! the clipboard. Every path is resolved against the worktree root, and a path that
//! leaves it is refused rather than followed.

mod clipboard;

#[cfg(test)]
mod tests;

pub use clipboard::{Clipboard, Memory, System, clipboard};

use std::path::{Component, Path, PathBuf};

use groove_types::{Error, ErrorKind, Result};

/// Writes `text` to `path` under `dir`.
pub fn save(dir: &Path, path: &str, text: &str) -> Result<()> {
    let full = inside(dir, path)?;
    std::fs::write(&full, text).map_err(|e| failed("write", path, &e))
}

/// The path under `dir` that `path` names.
fn inside(dir: &Path, path: &str) -> Result<PathBuf> {
    let relative = Path::new(path);
    let escapes = relative
        .components()
        .any(|part| matches!(part, Component::ParentDir | Component::RootDir));
    match escapes {
        true => Err(Error::new(
            ErrorKind::Invalid,
            format!("{path} leaves the worktree"),
        )),
        false => Ok(dir.join(relative)),
    }
}

fn failed(what: &str, path: &str, e: &std::io::Error) -> Error {
    Error::new(ErrorKind::Io, format!("cannot {what} {path}: {e}"))
}
