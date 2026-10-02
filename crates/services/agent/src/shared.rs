//! The team's shared repo: a copy of the branch it follows, kept under the app's data.

use std::path::{Path, PathBuf};

use groove_git::Git;
use groove_types::{Error, Result, SharedConfig};

/// The copy Groove keeps of the shared repo, and the marketplace it reads as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shared {
    pub path: PathBuf,
    pub marketplace: groove_skills::Marketplace,
}

/// Where the one copy lives.
pub fn copy_of(data: &Path) -> PathBuf {
    data.join("shared")
}

/// A fresh copy of the branch, in place of any copy before it, once it reads as a marketplace.
pub async fn join(data: &Path, shared: &SharedConfig) -> Result<Shared> {
    let path = copy_of(data);
    if path.exists() {
        std::fs::remove_dir_all(&path).map_err(|e| io(&path, e))?;
    }
    std::fs::create_dir_all(data).map_err(|e| io(data, e))?;
    if let Err(e) = Git::clone_head(&shared.url, &path, &shared.branch).await {
        let _ = std::fs::remove_dir_all(&path);
        return Err(unreached(shared, e.into()));
    }
    named(path)
}

/// The copy moved to the branch's head, or made when there is none yet.
pub async fn follow(data: &Path, shared: &SharedConfig) -> Result<Shared> {
    let path = copy_of(data);
    if !path.join(".git").exists() {
        return join(data, shared).await;
    }
    Git::at(&path).follow(&shared.branch).await?;
    named(path)
}

fn named(path: PathBuf) -> Result<Shared> {
    let marketplace = groove_skills::marketplace(&path)?;
    Ok(Shared { path, marketplace })
}

/// A clone that failed, with what to try when the URL asked for a password.
fn unreached(shared: &SharedConfig, e: Error) -> Error {
    match shared.url.starts_with("http") {
        true => Error::new(
            e.kind,
            format!("{}; for a private repo give its SSH URL", e.message),
        ),
        false => e,
    }
}

fn io(path: &Path, e: std::io::Error) -> Error {
    Error::new(
        groove_types::ErrorKind::Io,
        format!("{}: {e}", path.display()),
    )
}
