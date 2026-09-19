//! The task capability. Its slice of `AppState`, the operations on it, its events.

pub use groove_provider::{Github, Source, Token};
use groove_types::{Config, GithubConfig, Result, Task, TaskKey};

/// The `task` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    pub tasks: Vec<Task>,
    /// A read of the sources is out.
    pub reading: bool,
}

impl State {
    /// The tasks a read brought back, newest read wins.
    pub fn loaded(&mut self, tasks: Vec<Task>) {
        self.tasks = tasks;
        self.reading = false;
    }

    /// One task as its source now reports it.
    pub fn synced(&mut self, task: Task) {
        match self
            .tasks
            .iter_mut()
            .find(|one| one.short_id == task.short_id)
        {
            Some(held) => *held = task,
            None => self.tasks.push(task),
        }
    }

    pub fn get(&self, short_id: &str) -> Option<&Task> {
        self.tasks.iter().find(|task| task.short_id == short_id)
    }
}

/// The sources the config turns on. A source it does not name is not read.
pub fn sources(config: Option<&Config>) -> Vec<Source> {
    let github = config.and_then(|config| config.github.clone());
    github.into_iter().filter_map(github_source).collect()
}

fn github_source(config: GithubConfig) -> Option<Source> {
    Github::new(config).ok().map(Source::Github)
}

/// Every task the sources hold, in the order they answer.
pub async fn list(sources: &[Source]) -> Result<Vec<Task>> {
    let mut tasks = Vec::new();
    for source in sources {
        tasks.extend(source.list().await?);
    }
    Ok(tasks)
}

/// One task, read again from the source that owns it.
pub async fn fetch(sources: &[Source], key: &TaskKey) -> Result<Task> {
    let source = sources
        .iter()
        .find(|source| source.id() == key.provider())
        .ok_or_else(|| {
            groove_types::Error::invalid(format!("no source for {}", key.external_id()))
        })?;
    Ok(source.fetch(key).await?)
}

/// What the outside world tells this capability.
#[derive(Debug)]
pub enum Event {}

pub fn apply(_state: &mut State, event: Event) {
    match event {}
}
