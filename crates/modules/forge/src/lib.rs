//! The forge a repo lives on: the one MR a branch has, its CI and its threads.

mod error;
mod github;
mod gitlab;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use github::Github;
pub use gitlab::Gitlab;
pub use groove_token::Token;
use groove_types::{CiStatus, Forge, MrDetails, MrThread, Repo, ReviewMr, ReviewVerdict};

/// Every forge Groove speaks to.
pub enum Remote {
    Github(Github),
    Gitlab(Gitlab),
}

/// One MR as its forge holds it now: what a single call brings back.
#[derive(Debug)]
pub struct Snapshot {
    /// What the forge's own writes address it by.
    pub node: String,
    /// The commit the diff a note is posted on ends at.
    pub head: String,
    pub number: String,
    pub details: MrDetails,
    pub ci: Option<CiStatus>,
    pub threads: Vec<MrThread>,
}

/// What a new merge request says and where it goes.
pub struct Proposed<'a> {
    pub head: &'a str,
    pub base: Option<&'a str>,
    pub title: &'a str,
    pub body: &'a str,
}

/// A note posted on a line of the new side, as the forges take it.
#[derive(Clone, Copy)]
pub struct Posted<'a> {
    pub path: &'a str,
    /// The lines it covers, as a file numbers them.
    pub from: u32,
    pub to: u32,
    pub body: &'a str,
}

/// A review as the forges take it: its verdict, its words, and the notes it carries.
pub struct Verdict<'a> {
    pub said: ReviewVerdict,
    pub body: &'a str,
    pub notes: &'a [Posted<'a>],
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
    pub async fn find_mr(&self, repo: &Repo, branch: &str) -> Result<Option<Snapshot>> {
        match self {
            Remote::Github(github) => github.find_mr(repo, branch).await,
            Remote::Gitlab(gitlab) => gitlab.find_mr(repo, branch).await,
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
    pub async fn create_mr(&self, repo: &Repo, mr: Proposed<'_>) -> Result<Snapshot> {
        match self {
            Remote::Github(github) => github.create_mr(repo, mr).await,
            Remote::Gitlab(gitlab) => gitlab.create_mr(repo, mr).await,
        }
    }

    /// The viewer made an assignee of an MR it opened.
    pub async fn assign(&self, repo: &Repo, opened: &Snapshot) -> Result<()> {
        match self {
            Remote::Github(github) => github.assign(opened).await,
            Remote::Gitlab(gitlab) => gitlab.assign(repo, opened).await,
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
            Remote::Github(github) => github.close_mr(repo, number).await,
            Remote::Gitlab(gitlab) => gitlab.close_mr(repo, number).await,
        }
    }

    /// A note of this session posted on the merge request, at its own lines.
    pub async fn post_note(&self, repo: &Repo, number: &str, at: Posted<'_>) -> Result<()> {
        match self {
            Remote::Github(github) => github.note_on(repo, number, at).await,
            Remote::Gitlab(gitlab) => gitlab.note_on(repo, number, at).await,
        }
    }

    /// Words under a thread the forge holds.
    pub async fn reply_thread(
        &self,
        repo: &Repo,
        number: &str,
        thread: &str,
        body: &str,
    ) -> Result<()> {
        match self {
            Remote::Github(github) => github.reply_to(thread, body).await,
            Remote::Gitlab(gitlab) => gitlab.reply_to(repo, number, thread, body).await,
        }
    }

    /// A thread resolved, or opened again.
    pub async fn resolve_thread(&self, thread: &str, resolve: bool) -> Result<()> {
        match self {
            Remote::Github(github) => github.resolve(thread, resolve).await,
            Remote::Gitlab(gitlab) => gitlab.resolve(thread, resolve).await,
        }
    }

    /// A comment on the merge request itself, under no line.
    pub async fn comment(&self, repo: &Repo, number: &str, body: &str) -> Result<()> {
        match self {
            Remote::Github(github) => github.comment(repo, number, body).await,
            Remote::Gitlab(gitlab) => gitlab.comment(repo, number, body).await,
        }
    }

    /// A verdict on the merge request, with the notes it carries.
    pub async fn review(&self, repo: &Repo, number: &str, said: Verdict<'_>) -> Result<()> {
        match self {
            Remote::Github(github) => github.review(repo, number, said).await,
            Remote::Gitlab(gitlab) => gitlab.review(repo, number, said).await,
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
