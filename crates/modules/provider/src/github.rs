//! GitHub issues as tasks: the ones assigned to you, open, and on a project board.

mod query;
mod read;

use groove_http::{Client, Method};
use groove_types::{GithubConfig, Task, TaskKey};

use crate::token::{GhToken, Token};
use crate::{Error, Result};

pub struct Github {
    host: String,
    config: GithubConfig,
    client: Client,
    token: Token,
}

impl Github {
    /// A source called with the config's own token, or the one `gh` holds.
    pub fn new(config: GithubConfig) -> Result<Self> {
        let token = match config.token.clone() {
            Some(token) => Token::Fixed(token),
            None => Token::Gh(GhToken::new(&config.host)),
        };
        Self::with_token(config, token)
    }

    /// The same, called with a token of your own.
    pub fn with_token(config: GithubConfig, token: Token) -> Result<Self> {
        Ok(Self {
            host: config.host.clone(),
            config,
            client: Client::new()?,
            token,
        })
    }

    /// The endpoint every query goes to. A host that carries its own scheme is taken
    /// as it stands.
    fn url(&self) -> String {
        match self.host.as_str() {
            "github.com" => "https://api.github.com/graphql".to_string(),
            host if host.starts_with("http") => format!("{host}/api/graphql"),
            host => format!("https://{host}/api/graphql"),
        }
    }

    pub async fn list(&self) -> Result<Vec<Task>> {
        let reply = self
            .ask(&query::assigned(), serde_json::json!({ "after": null }))
            .await?;
        let nodes = reply["data"]["search"]["nodes"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        Ok(nodes
            .iter()
            .filter_map(|issue| read::task(issue, &self.host, &self.config))
            .collect())
    }

    pub async fn fetch(&self, key: &TaskKey) -> Result<Task> {
        let issue = self.issue(key).await?;
        read::task(&issue, &self.host, &self.config)
            .ok_or_else(|| Error::Invalid(format!("{} is on no project board", key.external_id())))
    }

    pub async fn body(&self, key: &TaskKey) -> Result<String> {
        let issue = self.issue(key).await?;
        Ok(issue["body"].as_str().unwrap_or_default().to_string())
    }

    /// One issue, by the owner, repo and number its key carries.
    async fn issue(&self, key: &TaskKey) -> Result<serde_json::Value> {
        let TaskKey::Github {
            owner,
            repo,
            number,
            ..
        } = key;
        let at = serde_json::json!({ "owner": owner, "repo": repo, "number": number });
        let reply = self.ask(&query::issue(), at).await?;
        let issue = reply["data"]["repository"]["issue"].clone();
        match issue.is_null() {
            true => Err(Error::Invalid(format!(
                "{} is not an issue {} knows",
                key.external_id(),
                self.host
            ))),
            false => Ok(issue),
        }
    }

    /// One GraphQL call, with the errors GitHub answers 200 with.
    async fn ask(&self, query: &str, variables: serde_json::Value) -> Result<serde_json::Value> {
        let body = serde_json::json!({ "query": query, "variables": variables });
        let reply: serde_json::Value = self
            .client
            .request(Method::POST, self.url())
            .json(&body)
            .send_authed_json(&self.token)
            .await?;
        match refusal(&reply) {
            Some(message) => Err(Error::Refused {
                host: self.host.clone(),
                message,
            }),
            None => Ok(reply),
        }
    }
}

/// What GitHub said was wrong with a query it still answered 200 to.
fn refusal(reply: &serde_json::Value) -> Option<String> {
    let errors = reply["errors"].as_array()?;
    let said: Vec<&str> = errors
        .iter()
        .filter_map(|one| one["message"].as_str())
        .collect();
    (!said.is_empty()).then(|| said.join("; "))
}
