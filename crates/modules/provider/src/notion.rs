//! Notion pages as tasks: the rows of one database that are yours and still open.

mod body;
mod read;
mod sprint;

use groove_http::{Client, Method};
use groove_types::{NotionConfig, StatusIntent, Task, TaskKey};

use crate::{Error, Result};

/// The API version this client speaks.
const VERSION: &str = "2022-06-28";

pub struct Notion {
    config: NotionConfig,
    client: Client,
    host: String,
    sprints: sprint::Sprints,
}

impl Notion {
    pub fn new(config: NotionConfig) -> Result<Self> {
        Self::at("https://api.notion.com", config)
    }

    /// The same, against another host, which the tests answer on.
    pub fn at(host: &str, config: NotionConfig) -> Result<Self> {
        Ok(Self {
            config,
            client: Client::new()?,
            host: host.to_string(),
            sprints: sprint::Sprints::default(),
        })
    }

    /// Every page of the database the filters leave.
    pub async fn list(&self) -> Result<Vec<Task>> {
        let url = self.query(&self.config.database_id);
        let sprints = self.running().await;
        let reply = self
            .ask(
                Method::POST,
                url,
                Some(read::filter(&self.config, &sprints)),
            )
            .await?;
        let pages = reply["results"].as_array().cloned().unwrap_or_default();
        Ok(pages
            .iter()
            .filter_map(|page| read::task(page, &self.config))
            .collect())
    }

    /// One page as a task, with its blocks as the body.
    pub async fn fetch(&self, key: &TaskKey) -> Result<crate::Fetched> {
        let page = self.page(key).await?;
        let task = read::task(&page, &self.config)
            .ok_or_else(|| Error::Invalid(format!("{} has no id of its own", self.id(key))))?;
        let blocks = self
            .ask(Method::GET, self.children(key), None)
            .await
            .map(|reply| body::text(&reply))
            .unwrap_or_default();
        Ok(crate::Fetched { task, body: blocks })
    }

    /// The pages of the sprint that is running, or nothing at all: a filter on a
    /// property the database lacks fails the whole query.
    async fn running(&self) -> Vec<String> {
        let Some(name) = self.config.sprint.clone() else {
            return Vec::new();
        };
        let now = groove_types::Timestamp::now();
        if let Some(held) = self.sprints.read(now) {
            return held;
        }
        let ids = self.current(&name).await.unwrap_or_default();
        self.sprints.keep(now, ids.clone());
        ids
    }

    /// The sprint database behind the relation, asked which of its rows is current.
    async fn current(&self, name: &str) -> Option<Vec<String>> {
        let database = self.database(&self.config.database_id).await.ok()?;
        let sprints = sprint::target(&database, name)?;
        let status = sprint::status_property(&self.database(&sprints).await.ok()?)?;
        let filter = serde_json::json!({
            "filter": {
                "property": status,
                "status": { "equals": self.config.sprint_status() }
            }
        });
        let reply = self
            .ask(Method::POST, self.query(&sprints), Some(filter))
            .await
            .ok()?;
        Some(sprint::ids(&reply))
    }

    async fn database(&self, id: &str) -> Result<serde_json::Value> {
        let url = format!("{}/v1/databases/{id}", self.host);
        self.ask(Method::GET, url, None).await
    }

    fn query(&self, database: &str) -> String {
        format!("{}/v1/databases/{database}/query", self.host)
    }

    /// Sets the page's status property to the label the config maps this intent to.
    pub async fn set_status(&self, key: &TaskKey, intent: StatusIntent) -> Result<String> {
        let name = self.config.properties.status.clone();
        let label = self
            .config
            .status_map
            .label(intent)
            .ok_or_else(|| Error::Invalid(format!("notion maps nothing to {intent:?}")))?
            .to_string();
        let value = serde_json::json!({ "status": { "name": label } });
        self.write(key, &name, value).await?;
        Ok(label)
    }

    /// Adds `hours` to what the page's own number property holds.
    pub async fn log_hours(&self, key: &TaskKey, hours: f32) -> Result<f32> {
        let name = self
            .config
            .properties
            .logged
            .clone()
            .ok_or_else(|| Error::Invalid("notion names no hours property".to_string()))?;
        let page = self.page(key).await?;
        let whole = read::number(&page, &name).unwrap_or_default() + hours;
        let value = serde_json::json!({ "number": whole });
        self.write(key, &name, value).await?;
        Ok(whole)
    }

    /// One property of one page.
    async fn write(&self, key: &TaskKey, name: &str, value: serde_json::Value) -> Result<()> {
        let body = serde_json::json!({ "properties": { name: value } });
        self.ask(Method::PATCH, self.url(key), Some(body)).await?;
        Ok(())
    }

    async fn page(&self, key: &TaskKey) -> Result<serde_json::Value> {
        self.ask(Method::GET, self.url(key), None).await
    }

    fn url(&self, key: &TaskKey) -> String {
        format!("{}/v1/pages/{}", self.host, self.id(key))
    }

    fn children(&self, key: &TaskKey) -> String {
        format!(
            "{}/v1/blocks/{}/children?page_size=100",
            self.host,
            self.id(key)
        )
    }

    fn id(&self, key: &TaskKey) -> String {
        match key {
            TaskKey::Notion { page_id } => page_id.clone(),
            other => other.external_id().to_string(),
        }
    }

    /// One call, with the token and the version every call carries.
    async fn ask(
        &self,
        method: Method,
        url: String,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut request = self
            .client
            .request(method, url)
            .bearer(self.config.token.clone())
            .header("Notion-Version", VERSION);
        if let Some(body) = &body {
            request = request.json(body);
        }
        let reply: serde_json::Value = request.send_json().await?;
        match reply["object"].as_str() {
            Some("error") => Err(Error::Refused {
                host: "notion".to_string(),
                message: message(&reply),
            }),
            _ => Ok(reply),
        }
    }
}

/// What Notion said was wrong.
fn message(reply: &serde_json::Value) -> String {
    reply["message"]
        .as_str()
        .unwrap_or("the call was refused")
        .to_string()
}
