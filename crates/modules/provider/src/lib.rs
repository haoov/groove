//! Where a task comes from: read here, written back only for the status and the hours.

mod error;
mod github;
mod notion;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use github::Github;
pub use groove_token::Token;
use groove_types::{StatusIntent, Task, TaskKey};
pub use notion::Notion;

/// The label the source's map gives this intent, or why it gives none.
fn mapped(map: &groove_types::StatusMap, intent: StatusIntent, source: &str) -> Result<String> {
    let label = map.label(intent).map(str::to_string);
    label.ok_or_else(|| Error::Invalid(format!("{source} maps nothing to {intent:?}")))
}

/// The property the source logs hours in, or why it names none.
fn hours_named(properties: &groove_types::PropertyNames, source: &str) -> Result<String> {
    let named = properties.logged.clone();
    named.ok_or_else(|| Error::Invalid(format!("{source} names no hours property")))
}

/// The most pages one list reads before it stops.
const PAGES_MAX: usize = 20;

/// Every source Groove reads.
pub enum Source {
    Github(Github),
    Notion(Notion),
}

/// One task as its source holds it, with the text the overview shows.
pub struct Fetched {
    pub task: Task,
    pub body: String,
}

impl Source {
    pub fn id(&self) -> groove_types::ProviderId {
        match self {
            Source::Github(_) => groove_types::ProviderId::Github,
            Source::Notion(_) => groove_types::ProviderId::Notion,
        }
    }

    /// Every task the source says is yours and open.
    pub async fn list(&self) -> Result<Vec<Task>> {
        match self {
            Source::Github(github) => github.list().await,
            Source::Notion(notion) => notion.list().await,
        }
    }

    /// One task and its body, read again from its source.
    pub async fn fetch(&self, key: &TaskKey) -> Result<Fetched> {
        match self {
            Source::Github(github) => github.fetch(key).await,
            Source::Notion(notion) => notion.fetch(key).await,
        }
    }

    /// The body a new task starts from, when the source holds one.
    pub async fn template(&self) -> Result<Option<String>> {
        match self {
            Source::Github(_) => Ok(None),
            Source::Notion(notion) => notion.template().await,
        }
    }

    /// Sets the task's status to what the source calls this intent. Returns that label.
    pub async fn set_status(&self, key: &TaskKey, intent: StatusIntent) -> Result<String> {
        match self {
            Source::Github(github) => github.set_status(key, intent).await,
            Source::Notion(notion) => notion.set_status(key, intent).await,
        }
    }

    /// Adds hours to what the source holds against the task. Returns its new total.
    pub async fn log_hours(&self, key: &TaskKey, hours: f32) -> Result<f32> {
        match self {
            Source::Github(github) => github.log_hours(key, hours).await,
            Source::Notion(notion) => notion.log_hours(key, hours).await,
        }
    }

    /// The properties the source holds, for its names to be mapped onto.
    pub async fn schema(&self) -> Result<Vec<groove_types::Property>> {
        match self {
            Source::Github(github) => github.schema().await,
            Source::Notion(notion) => notion.schema().await,
        }
    }
}
