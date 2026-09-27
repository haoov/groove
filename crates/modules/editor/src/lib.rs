//! The files of one worktree and the clipboard; a path that leaves the worktree is refused.

mod clipboard;

#[cfg(test)]
mod tests;

pub use clipboard::{Clipboard, Memory, clipboard};

use std::path::{Component, Path, PathBuf};

use groove_types::{Error, ErrorKind, Result};

/// Writes `text` to `path` under `dir`.
pub fn save(dir: &Path, path: &str, text: &str) -> Result<()> {
    let full = inside(dir, path)?;
    std::fs::write(&full, text).map_err(|e| failed("write", path, &e))
}

/// An empty file, or a directory, where nothing stands yet.
pub fn create(dir: &Path, path: &str, folder: bool) -> Result<()> {
    let full = inside(dir, path)?;
    if full.exists() {
        return Err(Error::new(
            ErrorKind::Conflict,
            format!("{path} is already there"),
        ));
    }
    if folder {
        return std::fs::create_dir_all(&full).map_err(|e| failed("make", path, &e));
    }
    parent(&full, path)?;
    std::fs::write(&full, "").map_err(|e| failed("make", path, &e))
}

/// One path moved to another, which must not stand yet.
pub fn rename(dir: &Path, from: &str, to: &str) -> Result<()> {
    let (old, new) = ends(dir, from, to)?;
    std::fs::rename(&old, &new).map_err(|e| failed("rename", from, &e))
}

/// One path copied to another, a directory with everything under it.
pub fn copy(dir: &Path, from: &str, to: &str) -> Result<()> {
    let (old, new) = ends(dir, from, to)?;
    match old.is_dir() {
        true => copy_dir(&old, &new).map_err(|e| failed("copy", from, &e)),
        false => std::fs::copy(&old, &new)
            .map(|_| ())
            .map_err(|e| failed("copy", from, &e)),
    }
}

/// One path taken away, a directory with everything under it.
pub fn delete(dir: &Path, path: &str) -> Result<()> {
    let full = inside(dir, path)?;
    match full.is_dir() {
        true => std::fs::remove_dir_all(&full).map_err(|e| failed("delete", path, &e)),
        false => std::fs::remove_file(&full).map_err(|e| failed("delete", path, &e)),
    }
}

/// Both ends of a move or a copy: the one that must be there, and the one that must not.
fn ends(dir: &Path, from: &str, to: &str) -> Result<(PathBuf, PathBuf)> {
    let old = inside(dir, from)?;
    let new = inside(dir, to)?;
    if !old.exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("{from} is not there"),
        ));
    }
    if new.exists() {
        return Err(Error::new(
            ErrorKind::Conflict,
            format!("{to} is already there"),
        ));
    }
    parent(&new, to)?;
    Ok((old, new))
}

/// The directories a path needs above it.
fn parent(full: &Path, path: &str) -> Result<()> {
    let Some(parent) = full.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|e| failed("make", path, &e))
}

/// A directory and everything under it, onto a path that stands nowhere yet.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)?.flatten() {
        let (here, there) = (entry.path(), to.join(entry.file_name()));
        match entry.file_type()?.is_dir() {
            true => copy_dir(&here, &there)?,
            false => {
                std::fs::copy(&here, &there)?;
            }
        }
    }
    Ok(())
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
