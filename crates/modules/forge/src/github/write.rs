//! The writes: a merge request opened, its text written again, or closed.

use groove_types::Repo;

use super::{Github, owner, query, read};
use crate::{Error, Posted, Proposed, Result, Snapshot, Verdict};

impl Github {
    /// One merge request opened on the repo's default branch, or on the base given.
    pub async fn open_new(&self, repo: &Repo, mr: Proposed<'_>) -> Result<Snapshot> {
        let (node, default) = self.repository(repo).await?;
        let base = mr.base.unwrap_or(&default);
        if base.is_empty() {
            return Err(Error::Invalid(format!(
                "{} names no branch to merge into",
                repo.slug()
            )));
        }
        let at = serde_json::json!({
            "repo": node,
            "base": base,
            "head": mr.head,
            "title": mr.title,
            "body": mr.body,
        });
        let reply = self.api.ask(&query::open(), at).await?;
        let (id, me) = self.me().await?;
        let opened = self.written(&reply, "createPullRequest", &me)?;
        let who = serde_json::json!({ "mr": opened.node, "who": [id] });
        self.api.ask(&query::assign(), who).await?;
        Ok(opened)
    }

    /// The title and the body of an MR written again.
    pub async fn edit_mr(
        &self,
        repo: &Repo,
        number: &str,
        title: &str,
        body: &str,
    ) -> Result<Snapshot> {
        let node = self.node_of(repo, number).await?;
        let at = serde_json::json!({ "mr": node, "title": title, "body": body });
        let reply = self.api.ask(&query::edit(), at).await?;
        let (_, me) = self.me().await?;
        self.written(&reply, "updatePullRequest", &me)
    }

    /// The MR closed, with nothing merged.
    pub async fn shut_mr(&self, repo: &Repo, number: &str) -> Result<Snapshot> {
        let node = self.node_of(repo, number).await?;
        let at = serde_json::json!({ "mr": node });
        let reply = self.api.ask(&query::shut(), at).await?;
        let (_, me) = self.me().await?;
        self.written(&reply, "closePullRequest", &me)
    }

    /// The repository's own id, and the branch it merges into by default.
    async fn repository(&self, repo: &Repo) -> Result<(String, String)> {
        let at = serde_json::json!({ "owner": owner(repo), "repo": repo.project });
        let reply = self.api.ask(&query::repository(), at).await?;
        let found = &reply["data"]["repository"];
        let node = read::text(&found["id"]);
        match node.is_empty() {
            true => Err(Error::Invalid(format!(
                "{} is not a repository {} knows",
                repo.slug(),
                self.host
            ))),
            false => Ok((node, read::text(&found["defaultBranchRef"]["name"]))),
        }
    }

    /// What a write addresses the MR by, read off the MR itself.
    async fn node_of(&self, repo: &Repo, number: &str) -> Result<String> {
        Ok(self.read_mr(repo, number).await?.node)
    }

    /// The token's own node id and login.
    async fn me(&self) -> Result<(String, String)> {
        let reply = self
            .api
            .ask(&query::viewer(), serde_json::json!({}))
            .await?;
        let viewer = &reply["data"]["viewer"];
        Ok((read::text(&viewer["id"]), read::text(&viewer["login"])))
    }

    /// The MR the mutation answered with, read as `me`.
    fn written(&self, reply: &serde_json::Value, mutation: &str, me: &str) -> Result<Snapshot> {
        let pr = &reply["data"][mutation]["pullRequest"];
        read::snapshot(pr, me).ok_or_else(|| {
            Error::Invalid(format!(
                "{} answered {mutation} with no merge request",
                self.host
            ))
        })
    }
}

impl Github {
    /// One thread opened on a file's new side, over the lines an anchor covers.
    pub async fn note_on(&self, repo: &Repo, number: &str, at: Posted<'_>) -> Result<()> {
        let node = self.node_of(repo, number).await?;
        let mut sent = serde_json::json!({
            "mr": node,
            "path": at.path,
            "to": at.to,
            "body": at.body,
        });
        let range = at.to > at.from;
        if range {
            sent["from"] = at.from.into();
        }
        self.api.ask(&query::note_on_line(range), sent).await?;
        Ok(())
    }

    /// Words under a thread that stands.
    pub async fn reply_to(&self, thread: &str, body: &str) -> Result<()> {
        let sent = serde_json::json!({ "thread": thread, "body": body });
        self.api.ask(&query::reply(), sent).await?;
        Ok(())
    }

    /// A thread resolved, or opened again.
    pub async fn resolve(&self, thread: &str, resolve: bool) -> Result<()> {
        let sent = serde_json::json!({ "thread": thread });
        let query = match resolve {
            true => query::resolve(),
            false => query::unresolve(),
        };
        self.api.ask(&query, sent).await?;
        Ok(())
    }
}

impl Github {
    /// A comment on the pull request itself.
    pub async fn comment(&self, repo: &Repo, number: &str, body: &str) -> Result<()> {
        let node = self.node_of(repo, number).await?;
        let sent = serde_json::json!({ "mr": node, "body": body });
        self.api.ask(&query::comment(), sent).await?;
        Ok(())
    }

    /// One review: its verdict, its words, and every note it carries as a thread.
    pub async fn review(&self, repo: &Repo, number: &str, said: Verdict<'_>) -> Result<()> {
        let node = self.node_of(repo, number).await?;
        let threads: Vec<serde_json::Value> = said.notes.iter().map(drafted).collect();
        let sent = serde_json::json!({
            "mr": node,
            "event": event(said.said),
            "body": said.body,
            "threads": threads,
        });
        self.api.ask(&query::review(), sent).await?;
        Ok(())
    }
}

/// One note as a thread the review opens; only a range names where it starts.
fn drafted(note: &Posted<'_>) -> serde_json::Value {
    let mut out = serde_json::json!({
        "path": note.path,
        "line": note.to,
        "side": "RIGHT",
        "body": note.body,
    });
    if note.to > note.from {
        out["startLine"] = note.from.into();
        out["startSide"] = "RIGHT".into();
    }
    out
}

/// The event a verdict is, in GitHub's own words.
fn event(said: groove_types::ReviewVerdict) -> &'static str {
    match said {
        groove_types::ReviewVerdict::Approve => "APPROVE",
        groove_types::ReviewVerdict::RequestChanges => "REQUEST_CHANGES",
        groove_types::ReviewVerdict::Comment => "COMMENT",
    }
}
