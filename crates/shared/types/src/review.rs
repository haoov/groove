//! The review queue: the MRs a forge asks this user to review, and where their reviewers stand.

use crate::{Forge, ReviewState, Reviewer, Timestamp};

/// An open MR where the user is a requested reviewer.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct ReviewMr {
    pub forge: Forge,
    pub project: String,
    pub iid: u64,
    pub title: String,
    pub author: String,
    pub source_branch: String,
    pub target_branch: String,
    pub draft: bool,
    pub web_url: String,
    pub updated_at: Timestamp,
    pub local_path: Option<String>,
    pub approved: bool,
    pub review: Option<ReviewState>,
}

impl ReviewMr {
    /// The ref the forge keeps the MR's head under, on the target repo, forks included.
    pub fn head_ref(&self) -> String {
        match self.forge {
            Forge::Github => format!("refs/pull/{}/head", self.iid),
            Forge::Gitlab => format!("refs/merge-requests/{}/head", self.iid),
        }
    }

    /// The session that reviews it, the same one every time it is opened.
    pub fn session_id(&self) -> String {
        let project = self
            .project
            .chars()
            .map(|c| match c.is_ascii_alphanumeric() {
                true => c.to_ascii_lowercase(),
                false => '-',
            })
            .collect::<String>();
        format!("review-{project}-{}", self.iid)
    }

    /// The pool's name for its repo: the host the page stands on, then the project.
    pub fn slug(&self) -> Option<String> {
        let rest = self.web_url.split_once("://")?.1;
        let host = rest.split('/').next().filter(|one| !one.is_empty())?;
        Some(format!("{host}/{}", self.project))
    }

    /// Whether the clone `entry` is its repo: the same host and the same project.
    pub fn held_by(&self, entry: &crate::PoolEntry) -> bool {
        self.slug().is_some_and(|slug| entry.slug == slug)
    }

    /// The pool's own name for its repo, or the URL it must be cloned from.
    pub fn repo_in(&self, pool: &[crate::PoolEntry]) -> Option<String> {
        let pooled = pool.iter().find(|entry| self.held_by(entry));
        pooled
            .map(|entry| entry.slug.clone())
            .or_else(|| self.clone_url())
    }

    /// Where its repo is cloned from, read off the page the MR stands on.
    pub fn clone_url(&self) -> Option<String> {
        let at = self
            .web_url
            .find("/-/merge_requests/")
            .or_else(|| self.web_url.find("/pull/"))?;
        let repo = self.web_url.get(..at)?;
        Some(format!("{repo}.git"))
    }
}

/// The review as the viewer sees it: still asked of them, else the review of every reviewer.
pub fn review_for(reviewers: &[Reviewer], approved: bool, asks_me: bool) -> Option<ReviewState> {
    match asks_me {
        true => Some(ReviewState::Requested),
        false => review_of(reviewers, approved),
    }
}

/// The most pressing verdict given, then approval, then a review still awaited.
pub fn review_of(reviewers: &[Reviewer], approved: bool) -> Option<ReviewState> {
    let has = |state| reviewers.iter().any(|one| one.state == state);
    match () {
        () if has(ReviewState::ChangesRequested) => Some(ReviewState::ChangesRequested),
        () if has(ReviewState::Commented) => Some(ReviewState::Commented),
        () if approved => Some(ReviewState::Approved),
        () => has(ReviewState::Requested).then_some(ReviewState::Requested),
    }
}
