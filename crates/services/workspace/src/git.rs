//! What git is asked for on the selected worktree.

use std::path::Path;

use groove_types::Result;

/// What the index holds, and the commit that turns it into history.
pub async fn stage(dir: &Path, paths: &[String]) -> Result<()> {
    groove_git::Git::at(dir).stage(paths).await?;
    Ok(())
}

pub async fn unstage(dir: &Path, paths: &[String]) -> Result<()> {
    groove_git::Git::at(dir).unstage(paths).await?;
    Ok(())
}

pub async fn discard(dir: &Path, paths: &[String]) -> Result<()> {
    groove_git::Git::at(dir).discard(paths).await?;
    Ok(())
}

pub async fn commit(dir: &Path, message: &str) -> Result<()> {
    groove_git::Git::at(dir).commit(message).await?;
    Ok(())
}

/// What the branch does against its remote and its base.
pub async fn push(dir: &Path, branch: &str) -> Result<()> {
    groove_git::Git::at(dir).push(branch).await?;
    Ok(())
}

pub async fn pull(dir: &Path) -> Result<()> {
    groove_git::Git::at(dir).pull().await?;
    Ok(())
}
