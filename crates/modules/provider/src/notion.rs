//! Notion pages as tasks: the rows of one database that are yours and still open.

pub(crate) mod body;
mod connect;
mod project;
mod read;
pub(crate) mod schema;
mod sprint;

use groove_http::{Client, Method};
use groove_types::{NotionConfig, StatusIntent, Task, TaskKey};

use crate::{Error, Result};

/// The API version this client speaks.
const VERSION: &str = "2022-06-28";
const HOST: &str = "https://api.notion.com";

pub struct Notion {
    config: NotionConfig,
    client: Client,
    host: String,
    sprints: sprint::Sprints,
    titles: project::Titles,
}

impl Notion {
    pub fn new(config: NotionConfig) -> Result<Self> {
        Self::at(HOST, config)
    }

    /// The same, against another host, which the tests answer on.
    pub fn at(host: &str, config: NotionConfig) -> Result<Self> {
        Ok(Self {
            config,
            client: Client::new()?,
            host: host.to_string(),
            sprints: sprint::Sprints::default(),
            titles: project::Titles::default(),
        })
    }

    /// Every page of the database the filters leave; none until the required names are given.
    pub async fn list(&self) -> Result<Vec<Task>> {
        let (Some(assignee), Some(sprint)) = (self.config.assignee(), self.config.sprint()) else {
            let missing = self.config.unmapped().join(" and ");
            return Err(Error::Invalid(format!("name the Notion {missing} first")));
        };
        let url = self.query(&self.config.database_id);
        let sprints = self.running(sprint).await?;
        let mut filter = read::filter(&self.config, (assignee, sprint), &sprints);
        let mut tasks = Vec::new();
        for _ in 0..crate::PAGES_MAX {
            let reply = self
                .ask(Method::POST, url.clone(), Some(filter.clone()))
                .await?;
            for page in reply["results"].as_array().into_iter().flatten() {
                if let Some(task) = read::task(page, &self.config) {
                    tasks.push(self.placed(task, page).await);
                }
            }
            if reply["has_more"].as_bool() != Some(true) {
                break;
            }
            filter["start_cursor"] = reply["next_cursor"].clone();
        }
        Ok(tasks)
    }

    /// The task with the project its page names.
    async fn placed(&self, mut task: Task, page: &serde_json::Value) -> Task {
        task.project = match project::held(page, self.config.properties.project.as_deref()) {
            Some(project::Held::Named(name)) => Some(name),
            Some(project::Held::Page(id)) => self.titled(&id).await,
            None => None,
        };
        task
    }

    /// A project page's title, read once.
    async fn titled(&self, id: &str) -> Option<String> {
        if let Some(title) = self.titles.get(id) {
            return Some(title);
        }
        let url = format!("{}/v1/pages/{id}", self.host);
        let page = self.ask(Method::GET, url, None).await.ok()?;
        let title = read::title(&page)?;
        self.titles.keep(id, &title);
        Some(title)
    }

    /// One page as a task, with its blocks as the body.
    pub async fn fetch(&self, key: &TaskKey) -> Result<crate::Fetched> {
        let page = self.page(key).await?;
        let task = read::task(&page, &self.config)
            .ok_or_else(|| Error::Invalid(format!("{} has no id of its own", self.id(key))))?;
        let task = self.placed(task, &page).await;
        let blocks = self.tree(&self.id(key), 0).await;
        let body = blocks.map(|one| body::text(&one)).unwrap_or_default();
        Ok(crate::Fetched { task, body })
    }

    /// The page the config names as the template for a new task.
    pub async fn template(&self) -> Result<Option<String>> {
        let Some(page) = self.config.task_template_page_id.clone() else {
            return Ok(None);
        };
        Ok(Some(body::text(&self.tree(&page, 0).await?)))
    }

    /// A block's children, and theirs down to `body::DEEP`; a child that fails to read has none.
    async fn tree(&self, id: &str, depth: usize) -> Result<Vec<body::Node>> {
        let url = format!("{}/v1/blocks/{id}/children?page_size=100", self.host);
        let reply = self.ask(Method::GET, url, None).await?;
        let blocks = reply["results"].as_array().cloned().unwrap_or_default();
        let mut out = Vec::with_capacity(blocks.len());
        for block in blocks {
            let children = match (body::opens(&block, depth), block["id"].as_str()) {
                (true, Some(child)) => Box::pin(self.tree(child, depth + 1)).await,
                _ => Ok(Vec::new()),
            };
            let children = children.unwrap_or_default();
            out.push(body::Node { block, children });
        }
        Ok(out)
    }

    /// The pages of the running sprint behind the `name` relation; a failed read is not kept.
    async fn running(&self, name: &str) -> Result<Vec<String>> {
        let now = groove_types::Timestamp::now();
        if let Some(held) = self.sprints.read(now) {
            return Ok(held);
        }
        let ids = self.current(name).await?;
        self.sprints.keep(now, ids.clone());
        Ok(ids)
    }

    /// The sprint database behind the relation, asked which of its rows is current; none without one.
    async fn current(&self, name: &str) -> Result<Vec<String>> {
        let database = self.database(&self.config.database_id).await?;
        let Some(sprints) = sprint::target(&database, name) else {
            return Ok(Vec::new());
        };
        let held = self.database(&sprints).await?;
        let Some(status) = sprint::status_property(&held) else {
            return Ok(Vec::new());
        };
        let filter = serde_json::json!({
            "filter": {
                "property": status,
                "status": { "equals": self.config.sprint_status() }
            }
        });
        let reply = self
            .ask(Method::POST, self.query(&sprints), Some(filter))
            .await?;
        Ok(sprint::ids(&reply))
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
        let label = crate::mapped(&self.config.status_map, intent, "notion")?;
        let value = serde_json::json!({ "status": { "name": label } });
        self.write(key, &name, value).await?;
        Ok(label)
    }

    /// Adds `hours` to what the page's own number property holds.
    pub async fn log_hours(&self, key: &TaskKey, hours: f32) -> Result<f32> {
        let name = crate::hours_named(&self.config.properties, "notion")?;
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
