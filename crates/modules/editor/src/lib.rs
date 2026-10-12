//! The files of one worktree and the clipboard; a path that leaves the worktree is refused.

mod clipboard;
mod files;

#[cfg(test)]
mod tests;

pub use clipboard::{Clipboard, Memory, clipboard};

use std::path::{Path, PathBuf};

use groove_types::{Error, ErrorKind, Result};

/// Writes `text` to `path` under `dir`.
pub async fn save(dir: &Path, path: &str, text: &str) -> Result<()> {
    let (dir, path, text) = (dir.to_path_buf(), path.to_string(), text.to_string());
    off(move || files::save(&dir, &path, &text)).await
}

/// An empty file, or a directory, where nothing stands yet.
pub async fn create(dir: &Path, path: &str, folder: bool) -> Result<()> {
    let (dir, path) = (dir.to_path_buf(), path.to_string());
    off(move || files::create(&dir, &path, folder)).await
}

/// One path moved to another, which must not stand yet.
pub async fn rename(dir: &Path, from: &str, to: &str) -> Result<()> {
    let (dir, from, to) = owned(dir, from, to);
    off(move || files::rename(&dir, &from, &to)).await
}

/// One path copied to another, a directory with everything under it.
pub async fn copy(dir: &Path, from: &str, to: &str) -> Result<()> {
    let (dir, from, to) = owned(dir, from, to);
    off(move || files::copy(&dir, &from, &to)).await
}

/// One path taken away, a directory with everything under it.
pub async fn delete(dir: &Path, path: &str) -> Result<()> {
    let (dir, path) = (dir.to_path_buf(), path.to_string());
    off(move || files::delete(&dir, &path)).await
}

fn owned(dir: &Path, from: &str, to: &str) -> (PathBuf, String, String) {
    (dir.to_path_buf(), from.to_string(), to.to_string())
}

async fn off(work: impl FnOnce() -> Result<()> + Send + 'static) -> Result<()> {
    match tokio::task::spawn_blocking(work).await {
        Ok(done) => done,
        Err(e) => Err(Error::new(ErrorKind::Io, e.to_string())),
    }
}
