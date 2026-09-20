//! Where a task comes from. Groove reads a task; only the agent writes one back,
//! except the status and the hours Groove itself recorded.

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

/// Every source Groove reads. A new provider is a new arm, and the compiler asks for
/// it everywhere at once.
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
}
