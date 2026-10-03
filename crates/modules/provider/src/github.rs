//! GitHub issues as tasks: the ones assigned to you, open, and on a project board.

mod query;
mod read;
pub(crate) mod schema;

use groove_http::Graphql;
use groove_token::{Cli, Token};
use groove_types::{Forge, GithubConfig, StatusIntent, Task, TaskKey};

use crate::{Error, Result};

pub struct Github {
    host: String,
    config: GithubConfig,
    api: Graphql<Token>,
}

impl Github {
    /// A source called with the config's own token, or the one `gh` holds.
    pub fn new(config: GithubConfig) -> Result<Self> {
        let token = Token::configured(config.token.clone(), Cli::Gh, &config.host);
        Self::with_token(config, token)
    }

    /// The same, called with a token of your own.
    pub fn with_token(config: GithubConfig, token: Token) -> Result<Self> {
        Ok(Self {
            host: config.host.clone(),
            api: Graphql::new(Forge::graphql(&config.host), token)?,
            config,
        })
    }

    /// The source on `host`, once the token `gh` holds for it is answered.
    pub async fn connect(host: &str) -> Result<GithubConfig> {
        let config = GithubConfig::bare(host);
        let github = Self::new(config.clone())?;
        github.ask(query::VIEWER, serde_json::json!({})).await?;
        Ok(config)
    }

    /// Every issue assigned to the viewer, page after page.
    pub async fn list(&self) -> Result<Vec<Task>> {
        let (mut tasks, mut after) = (Vec::new(), serde_json::Value::Null);
        for _ in 0..crate::PAGES_MAX {
            let reply = self
                .ask(&query::assigned(), serde_json::json!({ "after": after }))
                .await?;
            let search = &reply["data"]["search"];
            let nodes = search["nodes"].as_array().cloned().unwrap_or_default();
            let read = nodes
                .iter()
                .filter_map(|issue| read::task(issue, &self.host, &self.config));
            tasks.extend(read);
            if search["pageInfo"]["hasNextPage"].as_bool() != Some(true) {
                break;
            }
            after = search["pageInfo"]["endCursor"].clone();
        }
        Ok(tasks)
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
        } = key
        else {
            return Err(Error::Invalid(format!(
                "{} is not an issue of a repository",
                key.external_id()
            )));
        };
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

    async fn ask(&self, query: &str, variables: serde_json::Value) -> Result<serde_json::Value> {
        Ok(self.api.ask(query, variables).await?)
    }
}
