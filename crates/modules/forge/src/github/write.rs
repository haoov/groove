//! The writes: a merge request opened, its text written again, or closed.

use groove_types::Repo;

use super::{Github, owner, query, read};
use crate::{Error, Result, Snapshot};

/// What a new merge request says and where it goes.
pub struct Proposed<'a> {
    pub head: &'a str,
    pub base: Option<&'a str>,
    pub title: &'a str,
    pub body: &'a str,
}

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
        self.written(&reply, "createPullRequest")
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
        self.written(&reply, "updatePullRequest")
    }

    /// The MR closed, with nothing merged.
    pub async fn shut_mr(&self, repo: &Repo, number: &str) -> Result<Snapshot> {
        let node = self.node_of(repo, number).await?;
        let at = serde_json::json!({ "mr": node });
        let reply = self.api.ask(&query::shut(), at).await?;
        self.written(&reply, "closePullRequest")
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

    /// The MR the mutation answered with.
    fn written(&self, reply: &serde_json::Value, mutation: &str) -> Result<Snapshot> {
        let me = read::text(&reply["data"]["viewer"]["login"]);
        let pr = &reply["data"][mutation]["pullRequest"];
        read::snapshot(pr, &me).ok_or_else(|| {
            Error::Invalid(format!(
                "{} answered {mutation} with no merge request",
                self.host
            ))
        })
    }
}
