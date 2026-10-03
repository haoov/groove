//! The writes: a merge request opened, written again or closed, and what is said on it.

use groove_types::{Repo, ReviewVerdict};

use super::{Gitlab, path, query, read};
use crate::{Error, Posted, Proposed, Result, Snapshot, Verdict};

impl Gitlab {
    /// One MR opened from the worktree's branch into the base it names.
    pub async fn create_mr(&self, repo: &Repo, mr: Proposed<'_>) -> Result<Snapshot> {
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
        self.written(&reply, "mergeRequestCreate").await
    }

    /// The viewer made an assignee of the MR it opened.
    pub async fn assign(&self, repo: &Repo, opened: &Snapshot) -> Result<()> {
        let at = serde_json::json!({
            "path": path(repo),
            "iid": opened.number,
            "who": [opened.details.author],
        });
        self.sent(&query::assign(), at, "mergeRequestSetAssignees")
            .await
    }

    /// One mutation sent, refused when its answer names errors.
    async fn sent(&self, query: &str, at: serde_json::Value, mutation: &str) -> Result<()> {
        let reply = self.api.ask(query, at).await?;
        match refused(&reply["data"][mutation]) {
            Some(message) => Err(Error::Refused {
                host: self.host.clone(),
                message,
            }),
            None => Ok(()),
        }
    }

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
        self.written(&reply, "mergeRequestUpdate").await
    }

    /// The MR closed, with nothing merged.
    pub async fn close_mr(&self, repo: &Repo, number: &str) -> Result<Snapshot> {
        let at = serde_json::json!({ "path": path(repo), "iid": number });
        let reply = self.api.ask(&query::close(), at).await?;
        self.written(&reply, "mergeRequestUpdate").await
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
        self.sent(&query::note_on_line(range), sent, "createLatestDiffNote")
            .await
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
        self.sent(&query::reply(), sent, "createNote").await
    }

    /// A discussion resolved, or opened again.
    pub async fn resolve(&self, thread: &str, resolve: bool) -> Result<()> {
        let sent = serde_json::json!({ "thread": thread, "resolve": resolve });
        self.sent(&query::resolve(), sent, "discussionToggleResolve")
            .await
    }

    /// A comment on the merge request itself, under no discussion.
    pub async fn comment(&self, repo: &Repo, number: &str, body: &str) -> Result<()> {
        let mr = self.read_mr(repo, number).await?;
        let sent = serde_json::json!({ "mr": mr.node, "body": body });
        self.sent(&query::comment(), sent, "createNote").await
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

    /// What the verdict takes: a mutation for changes, and a REST call to approve or comment.
    async fn verdict(&self, repo: &Repo, number: &str, said: ReviewVerdict) -> Result<()> {
        match said {
            ReviewVerdict::Comment => {
                let url = reviewed_url(&self.host, repo, number);
                Ok(self.api.beside().post(&url).await?)
            }
            ReviewVerdict::RequestChanges => {
                let sent = serde_json::json!({ "path": path(repo), "iid": number });
                self.sent(
                    &query::request_changes(),
                    sent,
                    "mergeRequestRequestChanges",
                )
                .await
            }
            ReviewVerdict::Approve => {
                let url = approve_url(&self.host, repo, number);
                Ok(self.api.beside().post(&url).await?)
            }
        }
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

    /// The MR a mutation answered with, read as the viewer, whom a query of its own names.
    async fn written(&self, reply: &serde_json::Value, mutation: &str) -> Result<Snapshot> {
        let payload = &reply["data"][mutation];
        if let Some(refused) = refused(payload) {
            return Err(Error::Refused {
                host: self.host.clone(),
                message: refused,
            });
        }
        let who = self
            .api
            .ask(&query::viewer(), serde_json::json!({}))
            .await?;
        let me = self.viewer(&who);
        read::snapshot(&payload["mergeRequest"], &me, &self.host).ok_or_else(|| {
            Error::Invalid(format!(
                "{} answered {mutation} with no merge request",
                self.host
            ))
        })
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
    format!("{}/approve", super::mr_url(host, repo, number))
}

/// What marks the viewer's review given: a publish of no drafts, its state set.
fn reviewed_url(host: &str, repo: &Repo, number: &str) -> String {
    let publish = super::mr_url(host, repo, number);
    format!("{publish}/draft_notes/bulk_publish?reviewer_state=reviewed")
}
