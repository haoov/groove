//! GitLab merge requests: what a branch has, and what a write leaves behind.

mod query;
mod read;

use groove_http::Graphql;
use groove_token::Token;
use groove_types::{Forge, Repo, ReviewMr, ReviewVerdict};

use crate::github::Proposed;
use crate::{Error, Posted, Result, Snapshot, Verdict};

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

    /// One note on a line of the latest diff, which the MR's own head names.
    pub async fn note_on(&self, repo: &Repo, number: &str, at: Posted<'_>) -> Result<()> {
        let mr = self.read_mr(repo, number).await?;
        let mut sent = serde_json::json!({
            "mr": mr.node,
            "head": mr.head,
            "path": at.path,
            "from": at.from,
            "body": at.body,
        });
        let range = at.to > at.from;
        if range {
            sent["to"] = at.to.into();
        }
        self.api.ask(&query::note_on_line(range), sent).await?;
        Ok(())
    }

    /// Words under a discussion that stands.
    pub async fn reply_to(
        &self,
        repo: &Repo,
        number: &str,
        thread: &str,
        body: &str,
    ) -> Result<()> {
        let mr = self.read_mr(repo, number).await?;
        let sent = serde_json::json!({ "mr": mr.node, "thread": thread, "body": body });
        self.api.ask(&query::reply(), sent).await?;
        Ok(())
    }

    /// A discussion resolved, or opened again.
    pub async fn resolve(&self, thread: &str, resolve: bool) -> Result<()> {
        let sent = serde_json::json!({ "thread": thread, "resolve": resolve });
        self.api.ask(&query::resolve(), sent).await?;
        Ok(())
    }

    /// A comment on the merge request itself, under no discussion.
    pub async fn comment(&self, repo: &Repo, number: &str, body: &str) -> Result<()> {
        let mr = self.read_mr(repo, number).await?;
        let sent = serde_json::json!({ "mr": mr.node, "body": body });
        self.api.ask(&query::comment(), sent).await?;
        Ok(())
    }

    /// One review: every note it carries, its words, then the verdict itself.
    pub async fn review(&self, repo: &Repo, number: &str, said: Verdict<'_>) -> Result<()> {
        for note in said.notes {
            self.note_on(repo, number, *note).await?;
        }
        if !said.body.is_empty() {
            self.comment(repo, number, said.body).await?;
        }
        self.verdict(repo, number, said.said).await
    }

    /// What the verdict takes: a mutation for changes, and a REST call to approve.
    async fn verdict(&self, repo: &Repo, number: &str, said: ReviewVerdict) -> Result<()> {
        match said {
            ReviewVerdict::Comment => Ok(()),
            ReviewVerdict::RequestChanges => {
                let sent = serde_json::json!({ "path": path(repo), "iid": number });
                self.api.ask(&query::request_changes(), sent).await?;
                Ok(())
            }
            ReviewVerdict::Approve => {
                let url = approve_url(&self.host, repo, number);
                Ok(self.api.beside().post(&url).await?)
            }
        }
    }

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

/// What approves an MR, which GitLab keeps out of GraphQL.
fn approve_url(host: &str, repo: &Repo, number: &str) -> String {
    let project = urlencoding(&path(repo));
    let root = match host.starts_with("http") {
        true => host.to_string(),
        false => format!("https://{host}"),
    };
    format!("{root}/api/v4/projects/{project}/merge_requests/{number}/approve")
}

/// A path as one segment of a url: only the slash needs saying.
fn urlencoding(path: &str) -> String {
    path.replace('/', "%2F")
}

/// GitLab's own name for a repo: its group path and its project.
fn path(repo: &Repo) -> String {
    format!("{}/{}", repo.group_path, repo.project)
}
