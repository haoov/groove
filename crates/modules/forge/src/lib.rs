//! The forge a repo lives on: the one MR a branch has, its CI and its threads.
//! Groove reads these; the writes are the ones the user asks for.

mod error;
mod github;
mod gitlab;
mod store;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use github::{Github, Proposed};
pub use gitlab::Gitlab;
pub use groove_token::Token;
use groove_types::{CiStatus, Forge, MrDetails, MrThread, Repo, ReviewMr};
pub use store::Store;

/// Every forge Groove speaks to; a new one is a new arm the compiler asks for.
pub enum Remote {
    Github(Github),
    Gitlab(Gitlab),
}

/// One MR as its forge holds it now: what a single call brings back.
#[derive(Debug)]
pub struct Snapshot {
    /// What the forge's own writes address it by.
    pub node: String,
    pub number: String,
    pub details: MrDetails,
    pub ci: Option<CiStatus>,
    pub threads: Vec<MrThread>,
}

impl Remote {
    /// The forge that serves this repo's host.
    pub fn of(repo: &Repo) -> Result<Self> {
        Self::of_host(&repo.host)
    }

    pub fn kind(&self) -> Forge {
        match self {
            Remote::Github(_) => Forge::Github,
            Remote::Gitlab(_) => Forge::Gitlab,
        }
    }

    /// The open MR this branch is the source of, when the forge has one.
    pub async fn open_mr(&self, repo: &Repo, branch: &str) -> Result<Option<Snapshot>> {
        match self {
            Remote::Github(github) => github.open_mr(repo, branch).await,
            Remote::Gitlab(gitlab) => gitlab.open_mr(repo, branch).await,
        }
    }

    /// One MR by number, with its CI and its threads.
    pub async fn read_mr(&self, repo: &Repo, number: &str) -> Result<Snapshot> {
        match self {
            Remote::Github(github) => github.read_mr(repo, number).await,
            Remote::Gitlab(gitlab) => gitlab.read_mr(repo, number).await,
        }
    }

    /// A new MR, from the worktree's branch into the base it names.
    pub async fn open_mr_for(&self, repo: &Repo, mr: Proposed<'_>) -> Result<Snapshot> {
        match self {
            Remote::Github(github) => github.open_new(repo, mr).await,
            Remote::Gitlab(gitlab) => gitlab.open_new(repo, mr).await,
        }
    }

    /// Its title and its body written again.
    pub async fn edit_mr(
        &self,
        repo: &Repo,
        number: &str,
        title: &str,
        body: &str,
    ) -> Result<Snapshot> {
        match self {
            Remote::Github(github) => github.edit_mr(repo, number, title, body).await,
            Remote::Gitlab(gitlab) => gitlab.edit_mr(repo, number, title, body).await,
        }
    }

    /// The MR closed, with nothing merged.
    pub async fn close_mr(&self, repo: &Repo, number: &str) -> Result<Snapshot> {
        match self {
            Remote::Github(github) => github.shut_mr(repo, number).await,
            Remote::Gitlab(gitlab) => gitlab.shut_mr(repo, number).await,
        }
    }

    /// Every open MR the host asks this user to review.
    pub async fn review_queue(&self) -> Result<Vec<ReviewMr>> {
        match self {
            Remote::Github(github) => github.review_queue().await,
            Remote::Gitlab(gitlab) => gitlab.review_queue().await,
        }
    }

    /// The forge of a host, called with the token its CLI holds.
    pub fn of_host(host: &str) -> Result<Self> {
        match Forge::of_host(host) {
            Forge::Github => Ok(Remote::Github(Github::new(host)?)),
            Forge::Gitlab => Ok(Remote::Gitlab(Gitlab::new(host)?)),
        }
    }
}
