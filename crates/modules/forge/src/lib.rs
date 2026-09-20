//! The forge a repo lives on: the one MR a branch has, its CI and its threads.
//! Groove reads these; the writes are the ones the user asks for.

mod error;
mod github;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use github::Github;
use groove_token::Token;
use groove_types::{CiStatus, Forge, MrDetails, MrThread, Repo};

/// Every forge Groove speaks to; a new one is a new arm the compiler asks for.
pub enum Remote {
    Github(Github),
}

/// One MR as its forge holds it now: what a single call brings back.
#[derive(Debug)]
pub struct Snapshot {
    pub number: String,
    pub details: MrDetails,
    pub ci: Option<CiStatus>,
    pub threads: Vec<MrThread>,
}

impl Remote {
    /// The forge that serves this repo's host.
    pub fn of(repo: &Repo) -> Result<Self> {
        match Forge::of_host(&repo.host) {
            Forge::Github => Ok(Remote::Github(Github::new(&repo.host)?)),
            Forge::Gitlab => Err(Error::Invalid(format!(
                "{} is a GitLab host, which Groove cannot read yet",
                repo.host
            ))),
        }
    }

    /// The same, called with a token of your own.
    pub fn with_token(repo: &Repo, token: Token) -> Result<Self> {
        match Forge::of_host(&repo.host) {
            Forge::Github => Ok(Remote::Github(Github::with_token(&repo.host, token)?)),
            Forge::Gitlab => Err(Error::Invalid(format!("{} is a GitLab host", repo.host))),
        }
    }

    pub fn kind(&self) -> Forge {
        match self {
            Remote::Github(_) => Forge::Github,
        }
    }

    /// The open MR this branch is the source of, when the forge has one.
    pub async fn open_mr(&self, repo: &Repo, branch: &str) -> Result<Option<Snapshot>> {
        match self {
            Remote::Github(github) => github.open_mr(repo, branch).await,
        }
    }

    /// One MR by number, with its CI and its threads.
    pub async fn read_mr(&self, repo: &Repo, number: &str) -> Result<Snapshot> {
        match self {
            Remote::Github(github) => github.read_mr(repo, number).await,
        }
    }
}
