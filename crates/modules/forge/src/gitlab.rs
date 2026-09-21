//! GitLab merge requests: what a branch has, and what a write leaves behind.

mod query;
mod read;

use groove_http::Graphql;
use groove_token::Token;
use groove_types::{Forge, Repo, ReviewMr};

use crate::github::Proposed;
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

    /// One MR opened from the worktree's branch into the base it names.
    pub async fn open_new(&self, repo: &Repo, mr: Proposed<'_>) -> Result<Snapshot> {
        let base = match mr.base {
            Some(base) => base.to_string(),
            None => self.root_ref(repo).await?,
        };
        let at = serde_json::json!({
            "path": path(repo),
            "head": mr.head,
            "base": base,
            "title": mr.title,
            "body": mr.body,
        });
        let reply = self.api.ask(&query::open(), at).await?;
        self.written(&reply, "mergeRequestCreate")
    }

    /// The title and the body written again.
    pub async fn edit_mr(
        &self,
        repo: &Repo,
        number: &str,
        title: &str,
        body: &str,
    ) -> Result<Snapshot> {
        let at = serde_json::json!({
            "path": path(repo),
            "iid": number,
            "title": title,
            "body": body,
        });
        let reply = self.api.ask(&query::edit(), at).await?;
        self.written(&reply, "mergeRequestUpdate")
    }

    /// The MR closed, with nothing merged.
    pub async fn shut_mr(&self, repo: &Repo, number: &str) -> Result<Snapshot> {
        let at = serde_json::json!({ "path": path(repo), "iid": number });
        let reply = self.api.ask(&query::shut(), at).await?;
        self.written(&reply, "mergeRequestUpdate")
    }

    /// Every open MR the host asks this user to review, newest first.
    pub async fn review_queue(&self) -> Result<Vec<ReviewMr>> {
        let at = serde_json::json!({ "first": QUEUE_MAX });
        let reply = self.api.ask(&query::review_queue(), at).await?;
        let asked = &reply["data"]["currentUser"]["reviewRequestedMergeRequests"];
        Ok(read::nodes(asked).iter().filter_map(read::asked).collect())
    }

    /// The branch the project merges into by default.
    async fn root_ref(&self, repo: &Repo) -> Result<String> {
        let at = serde_json::json!({ "path": path(repo) });
        let reply = self.api.ask(&query::root_ref(), at).await?;
        let found = read::text(&reply["data"]["project"]["repository"]["rootRef"]);
        match found.is_empty() {
            true => Err(Error::Invalid(format!(
                "{} names no branch to merge into",
                repo.slug()
            ))),
            false => Ok(found),
        }
    }

    /// The MR a mutation answered with. Its own errors are the write's failure.
    fn written(&self, reply: &serde_json::Value, mutation: &str) -> Result<Snapshot> {
        let payload = &reply["data"][mutation];
        if let Some(refused) = refused(payload) {
            return Err(Error::Refused {
                host: self.host.clone(),
                message: refused,
            });
        }
        let me = self.viewer(reply);
        read::snapshot(&payload["mergeRequest"], &me, &self.host).ok_or_else(|| {
            Error::Invalid(format!(
                "{} answered {mutation} with no merge request",
                self.host
            ))
        })
    }

    fn viewer(&self, reply: &serde_json::Value) -> String {
        read::text(&reply["data"]["currentUser"]["username"])
    }
}

/// What a mutation says was wrong with a write it still answered 200 to.
fn refused(payload: &serde_json::Value) -> Option<String> {
    let errors = payload["errors"].as_array()?;
    let said: Vec<&str> = errors.iter().filter_map(|one| one.as_str()).collect();
    (!said.is_empty()).then(|| said.join("; "))
}

/// GitLab's own name for a repo: its group path and its project.
fn path(repo: &Repo) -> String {
    format!("{}/{}", repo.group_path, repo.project)
}
