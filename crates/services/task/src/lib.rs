//! The task capability. Its slice of `AppState`, the operations on it, its events.

mod order;
mod service;
mod timer;

pub use groove_plan::Placed;
pub use groove_provider::{Fetched, Github, Source, Token};
use groove_types::{Config, GithubConfig, Result, Task, TaskKey};

pub use order::{Planned, moved, ordered};
pub use service::Service;
pub use timer::{IDLE, Timer};

/// The `task` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    pub tasks: Vec<Task>,
    /// The body each read task carries, by short id.
    pub bodies: std::collections::BTreeMap<String, String>,
    /// A read of the sources is out.
    pub reading: bool,
    /// The tasks a read is out for.
    pub syncing: std::collections::BTreeSet<groove_types::ExternalId>,
    /// The order the user gave them.
    pub plan: Vec<Placed>,
    /// What each task has measured, and the clock that measures it.
    pub time: std::collections::BTreeMap<groove_types::ExternalId, groove_types::TimeSummary>,
    pub timer: Timer,
}

impl State {
    /// The tasks a read brought back, newest read wins.
    pub fn loaded(&mut self, tasks: Vec<Task>) {
        self.tasks = tasks;
        self.reading = false;
    }

    /// One task and its body as its source now reports them.
    pub fn synced(&mut self, read: Fetched) {
        let Fetched { task, body } = read;
        self.bodies.insert(task.short_id.clone(), body);
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

    pub fn by_external(&self, external_id: &groove_types::ExternalId) -> Option<&Task> {
        self.tasks
            .iter()
            .find(|task| task.external_id == *external_id)
    }

    pub fn body(&self, short_id: &str) -> Option<&str> {
        self.bodies.get(short_id).map(String::as_str)
    }

    /// What one task has measured, if anything.
    pub fn measured(&self, id: &groove_types::ExternalId) -> Option<groove_types::TimeSummary> {
        self.time.get(id).copied()
    }

    /// The tasks no session works, in the user's own order.
    pub fn planned<'a>(&'a self, waiting: &[&'a Task]) -> Vec<Planned<'a>> {
        ordered(&self.plan, waiting)
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

/// Sets one task's status to what its source calls this intent.
pub async fn set_status(
    sources: &[Source],
    key: &TaskKey,
    intent: groove_types::StatusIntent,
) -> Result<String> {
    Ok(source_of(sources, key)?.set_status(key, intent).await?)
}

/// Adds hours to what the source holds against one task. Returns its new total.
pub async fn log_hours(sources: &[Source], key: &TaskKey, hours: f32) -> Result<f32> {
    Ok(source_of(sources, key)?.log_hours(key, hours).await?)
}

/// One task and its body, read again from the source that owns it.
pub async fn fetch(sources: &[Source], key: &TaskKey) -> Result<Fetched> {
    Ok(source_of(sources, key)?.fetch(key).await?)
}

/// The source that owns the key, of the ones the config turned on.
fn source_of<'a>(sources: &'a [Source], key: &TaskKey) -> Result<&'a Source> {
    sources
        .iter()
        .find(|source| source.id() == key.provider())
        .ok_or_else(|| groove_types::Error::invalid(format!("no source for {}", key.external_id())))
}

/// What the outside world tells this capability.
#[derive(Debug)]
pub enum Event {}

pub fn apply(_state: &mut State, event: Event) {
    match event {}
}
