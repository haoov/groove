//! Where a task comes from. Groove reads a task; only the agent writes one back,
//! except the status and the hours Groove itself recorded.

mod error;
mod github;
mod token;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use github::Github;
use groove_types::{Task, TaskKey};
pub use token::Token;

/// Every source Groove reads. A new provider is a new arm, and the compiler asks for
/// it everywhere at once.
pub enum Source {
    Github(Github),
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
        }
    }

    /// Every task the source says is yours and open.
    pub async fn list(&self) -> Result<Vec<Task>> {
        match self {
            Source::Github(github) => github.list().await,
        }
    }

    /// One task and its body, read again from its source.
    pub async fn fetch(&self, key: &TaskKey) -> Result<Fetched> {
        match self {
            Source::Github(github) => github.fetch(key).await,
        }
    }

    /// Adds hours to what the source holds against the task. Returns its new total.
    pub async fn log_hours(&self, key: &TaskKey, hours: f32) -> Result<f32> {
        match self {
            Source::Github(github) => github.log_hours(key, hours).await,
        }
    }
}
