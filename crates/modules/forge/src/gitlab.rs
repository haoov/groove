//! GitLab merge requests: what a branch has, and what a write leaves behind.

mod query;
mod ranges;
mod read;
mod write;

use groove_http::Graphql;
use groove_token::Token;
use groove_types::{Forge, Repo, ReviewMr};

use crate::{Error, Result, Snapshot};

/// How many of the queue a call asks for.
const QUEUE_MAX: i64 = 50;

pub struct Gitlab {
    host: String,
    api: Graphql<Token>,
}

impl Gitlab {
    /// A client called with the token `glab` holds for the host.
    pub fn new(host: &str) -> Result<Self> {
        Self::with_token(host, Token::glab(host))
    }

    /// The same, called with a token of your own.
    pub fn with_token(host: &str, token: Token) -> Result<Self> {
        Ok(Self {
            host: host.to_string(),
            api: Graphql::new(Forge::graphql(host), token)?,
        })
    }

    /// The open MR this branch is the source of, when the host has one.
    pub async fn open_mr(&self, repo: &Repo, branch: &str) -> Result<Option<Snapshot>> {
        let at = serde_json::json!({ "path": path(repo), "branch": branch });
        let reply = self.api.ask(&query::by_branch(), at).await?;
        let nodes = read::nodes(&reply["data"]["project"]["mergeRequests"]);
        let Some(mr) = nodes.first() else {
            return Ok(None);
        };
        match read::snapshot(mr, &self.viewer(&reply), &self.host) {
            Some(read) => Ok(Some(self.spanned(repo, read).await)),
            None => Ok(None),
        }
    }

    /// One MR by number, with its pipeline and its discussions.
    pub async fn read_mr(&self, repo: &Repo, number: &str) -> Result<Snapshot> {
        let at = serde_json::json!({ "path": path(repo), "iid": number });
        let reply = self.api.ask(&query::by_iid(), at).await?;
        let mr = &reply["data"]["project"]["mergeRequest"];
        let read = read::snapshot(mr, &self.viewer(&reply), &self.host).ok_or_else(|| {
            Error::Invalid(format!(
                "{} has no merge request {number} on {}",
                self.host,
                repo.slug()
            ))
        })?;
        Ok(self.spanned(repo, read).await)
    }

    /// The read with each diff note's range; left as it is when the call fails.
    async fn spanned(&self, repo: &Repo, mut read: Snapshot) -> Snapshot {
        if !ranges::wanted(&read.threads) {
            return read;
        }
        let url = ranges::url(&self.host, repo, &read.number);
        if let Ok(discussions) = self.api.beside().get(&url).await {
            ranges::spanned(&mut read.threads, &discussions);
        }
        read
    }

    pub async fn review_queue(&self) -> Result<Vec<ReviewMr>> {
        let at = serde_json::json!({ "first": QUEUE_MAX });
        let reply = self.api.ask(&query::review_queue(), at).await?;
        let me = self.viewer(&reply);
        let asked = &reply["data"]["currentUser"]["reviewRequestedMergeRequests"];
        let mrs = read::nodes(asked).into_iter();
        Ok(mrs.filter_map(|mr| read::asked(&mr, &me)).collect())
    }

    fn viewer(&self, reply: &serde_json::Value) -> String {
        read::text(&reply["data"]["currentUser"]["username"])
    }
}

/// A path as one segment of a url: only the slash needs saying.
/// One merge request's REST address.
fn mr_url(host: &str, repo: &Repo, number: &str) -> String {
    let project = urlencoding(&path(repo));
    let root = groove_types::Forge::root(host);
    format!("{root}/api/v4/projects/{project}/merge_requests/{number}")
}

fn urlencoding(path: &str) -> String {
    path.replace('/', "%2F")
}

/// GitLab's own name for a repo: its group path and its project.
fn path(repo: &Repo) -> String {
    format!("{}/{}", repo.group_path, repo.project)
}
