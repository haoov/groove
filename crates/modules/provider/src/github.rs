//! GitHub issues as tasks: the ones assigned to you, open, and on a project board.

mod query;
mod read;

use groove_http::{Client, Method};
use groove_types::{GithubConfig, StatusIntent, Task, TaskKey};

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

    pub async fn fetch(&self, key: &TaskKey) -> Result<crate::Fetched> {
        let issue = self.issue(key).await?;
        let task = read::task(&issue, &self.host, &self.config).ok_or_else(|| {
            Error::Invalid(format!("{} is on no project board", key.external_id()))
        })?;
        Ok(crate::Fetched {
            task,
            body: issue["body"].as_str().unwrap_or_default().to_string(),
        })
    }

    /// Sets the board's status field to the label the config maps this intent to.
    pub async fn set_status(&self, key: &TaskKey, intent: StatusIntent) -> Result<String> {
        let name = self.config.properties.status.clone();
        let label = self
            .config
            .status_map
            .label(intent)
            .ok_or_else(|| Error::Invalid(format!("{} maps nothing to {intent:?}", self.host)))?
            .to_string();
        let issue = self.issue(key).await?;
        let item = read::item(&issue).ok_or_else(|| self.off_board(key))?;
        let ids = read::ids(item, &name).ok_or_else(|| self.no_field(&name))?;
        let option = read::option(item, &name, &label)
            .ok_or_else(|| Error::Invalid(format!("{name} has no option called {label}")))?;
        let at = serde_json::json!({
            "project": ids.project,
            "item": ids.item,
            "field": ids.field,
            "option": option,
        });
        self.ask(&query::set_select(), at).await?;
        Ok(label)
    }

    /// Adds `hours` to what the board's own field holds against the issue.
    pub async fn log_hours(&self, key: &TaskKey, hours: f32) -> Result<f32> {
        let name = self
            .config
            .properties
            .logged
            .clone()
            .ok_or_else(|| Error::Invalid(format!("{} names no hours field", self.host)))?;
        let issue = self.issue(key).await?;
        let item = read::item(&issue).ok_or_else(|| self.off_board(key))?;
        let ids = read::ids(item, &name).ok_or_else(|| self.no_field(&name))?;
        let whole = read::number(item, &name).unwrap_or_default() + hours;
        let at = serde_json::json!({
            "project": ids.project,
            "item": ids.item,
            "field": ids.field,
            "value": whole,
        });
        self.ask(&query::set_number(), at).await?;
        Ok(whole)
    }

    fn off_board(&self, key: &TaskKey) -> Error {
        Error::Invalid(format!("{} is on no project board", key.external_id()))
    }

    fn no_field(&self, name: &str) -> Error {
        Error::Invalid(format!("the board has no field called {name}"))
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
