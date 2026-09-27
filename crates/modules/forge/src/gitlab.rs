//! GitLab merge requests: what a branch has, and what a write leaves behind.

mod query;
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
        Ok(read::snapshot(mr, &self.viewer(&reply), &self.host))
    }

    /// One MR by number, with its pipeline and its discussions.
    pub async fn read_mr(&self, repo: &Repo, number: &str) -> Result<Snapshot> {
        let at = serde_json::json!({ "path": path(repo), "iid": number });
        let reply = self.api.ask(&query::by_iid(), at).await?;
        let mr = &reply["data"]["project"]["mergeRequest"];
        read::snapshot(mr, &self.viewer(&reply), &self.host).ok_or_else(|| {
            Error::Invalid(format!(
                "{} has no merge request {number} on {}",
                self.host,
                repo.slug()
            ))
        })
    }

    pub async fn review_queue(&self) -> Result<Vec<ReviewMr>> {
        let at = serde_json::json!({ "first": QUEUE_MAX });
        let reply = self.api.ask(&query::review_queue(), at).await?;
        let asked = &reply["data"]["currentUser"]["reviewRequestedMergeRequests"];
        Ok(read::nodes(asked).iter().filter_map(read::asked).collect())
    }

    fn viewer(&self, reply: &serde_json::Value) -> String {
        read::text(&reply["data"]["currentUser"]["username"])
    }
}

/// A path as one segment of a url: only the slash needs saying.
fn urlencoding(path: &str) -> String {
    path.replace('/', "%2F")
}

/// GitLab's own name for a repo: its group path and its project.
fn path(repo: &Repo) -> String {
    format!("{}/{}", repo.group_path, repo.project)
}
