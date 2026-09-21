//! GitHub pull requests: the one a branch has, and all a single read brings back.

mod query;
mod read;
mod write;

pub use write::Proposed;

use groove_http::Graphql;
use groove_token::Token;
use groove_types::{Forge, Repo};

use crate::{Error, Result, Snapshot};

pub struct Github {
    host: String,
    api: Graphql<Token>,
}

impl Github {
    /// A client called with the token `gh` holds for the host.
    pub fn new(host: &str) -> Result<Self> {
        Self::with_token(host, Token::gh(host))
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
        let at = serde_json::json!({
            "owner": owner(repo),
            "repo": repo.project,
            "branch": branch,
        });
        let reply = self.api.ask(&query::by_branch(), at).await?;
        let nodes = &reply["data"]["repository"]["pullRequests"]["nodes"];
        Ok(read::snapshot(&nodes[0], &viewer(&reply)))
    }

    /// One MR by number, with its CI and its threads.
    pub async fn read_mr(&self, repo: &Repo, number: &str) -> Result<Snapshot> {
        let at = serde_json::json!({
            "owner": owner(repo),
            "repo": repo.project,
            "number": self.numbered(number)?,
        });
        let reply = self.api.ask(&query::by_number(), at).await?;
        let pr = &reply["data"]["repository"]["pullRequest"];
        read::snapshot(pr, &viewer(&reply)).ok_or_else(|| {
            Error::Invalid(format!(
                "{} has no merge request {number} on {}",
                self.host,
                repo.slug()
            ))
        })
    }

    fn host(&self) -> &str {
        &self.host
    }

    fn numbered(&self, number: &str) -> Result<i64> {
        number
            .parse()
            .map_err(|_| Error::Invalid(format!("{number} is not a merge request number")))
    }
}

/// Who is asking, as every reply says.
fn viewer(reply: &serde_json::Value) -> String {
    read::text(&reply["data"]["viewer"]["login"])
}

/// GitHub's owner: the last segment of the group path.
fn owner(repo: &Repo) -> &str {
    repo.group_path
        .rsplit('/')
        .next()
        .unwrap_or(&repo.group_path)
}
