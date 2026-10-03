//! The task capability: the tasks and their sources, the plan, the clock and hours, attention.

use std::sync::{Arc, PoisonError};

mod attention;
mod filing;
mod order;
mod referenced;
mod service;
mod timer;

#[cfg(test)]
mod tests;

pub use groove_browser::browse;
pub use groove_plan::Placed;
pub use groove_provider::{Fetched, Github, Notion, Source, Token};
use groove_types::{
    Attention, Config, ExternalId, GithubConfig, NotionConfig, ProviderId, Result, Session, Task,
    TaskKey, TimeSummary,
};

pub use attention::folded;
pub use filing::{Filing, filing};
pub use order::{Planned, moved, ordered};
pub use referenced::referenced;
pub use service::Service;
pub use timer::{IDLE, Timer, working};

/// The `task` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    pub tasks: Vec<Task>,
    /// The body each read task carries, by short id.
    pub bodies: std::collections::BTreeMap<String, String>,
    /// A read of the sources is out.
    pub reading: bool,
    /// The tasks a read is out for.
    pub syncing: std::collections::BTreeSet<ExternalId>,
    /// The order the user gave them.
    pub plan: Vec<Placed>,
    /// What needs the user, by task.
    pub attention: std::collections::BTreeMap<ExternalId, Vec<Attention>>,
    /// What each task has measured, and the clock that measures it.
    pub time: std::collections::BTreeMap<ExternalId, TimeSummary>,
    pub timer: Timer,
    /// The day the attention rules last read the tasks for.
    pub attended: Option<groove_types::Day>,
    built: Built,
}

/// The sources, built once for the config that turned them on.
#[derive(Default)]
struct Built(std::sync::Mutex<Option<Building>>);

/// The config it was built for, the sources, and why any of them could not be.
type Building = (SourceKey, Arc<Vec<Source>>, Vec<groove_types::Error>);

type SourceKey = (Option<GithubConfig>, Option<NotionConfig>);

impl std::fmt::Debug for Built {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Built")
    }
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

    pub fn by_external(&self, external_id: &ExternalId) -> Option<&Task> {
        self.tasks
            .iter()
            .find(|task| task.external_id == *external_id)
    }

    /// The task a session works, as the slice holds it.
    pub fn worked(&self, session: &Session) -> Option<&Task> {
        self.by_external(session.kind.task()?)
    }

    pub fn body(&self, short_id: &str) -> Option<&str> {
        self.bodies.get(short_id).map(String::as_str)
    }

    /// Why one task needs the user, if it does.
    pub fn needs(&self, id: &ExternalId) -> &[Attention] {
        self.attention.get(id).map_or(&[], Vec::as_slice)
    }

    /// Whether one task needs the user at all.
    pub fn asks(&self, id: &ExternalId) -> bool {
        !self.needs(id).is_empty()
    }

    /// The sources the config turns on, the same ones while the config stays the same.
    pub fn sources(&self, config: Option<&Config>) -> Arc<Vec<Source>> {
        let key = (
            config.and_then(|one| one.github.clone()),
            config.and_then(|one| one.notion.clone()),
        );
        let mut held = self.built.0.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((_, sources, _)) = held.as_ref().filter(|(at, ..)| *at == key) {
            return sources.clone();
        }
        let (sources, broken) = built(&key);
        let sources = Arc::new(sources);
        *held = Some((key, sources.clone(), broken));
        sources
    }

    /// Why a source the config turns on could not be built, as the last build found it.
    pub fn broken(&self) -> Vec<groove_types::Error> {
        let held = self.built.0.lock().unwrap_or_else(PoisonError::into_inner);
        held.as_ref()
            .map(|(.., broken)| broken.clone())
            .unwrap_or_default()
    }

    /// What every task has measured, as the ledger now holds it.
    pub fn timed(&mut self, time: Vec<(ExternalId, TimeSummary)>) {
        self.time = time.into_iter().collect();
    }

    /// What one task has measured, if anything.
    pub fn measured(&self, id: &ExternalId) -> Option<TimeSummary> {
        self.time.get(id).copied()
    }

    /// The tasks none of `worked` names.
    pub fn waiting(&self, worked: &[ExternalId]) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|task| !worked.contains(&task.external_id))
            .collect()
    }

    /// The tasks no session works, in the user's own order.
    pub fn planned<'a>(&'a self, waiting: &[&'a Task]) -> Vec<Planned<'a>> {
        ordered(&self.plan, waiting)
    }
}

/// The task sources this machine is set up for, in the order they are offered.
pub fn source_ids(config: Option<&Config>) -> Vec<ProviderId> {
    let Some(config) = config else {
        return Vec::new();
    };
    ProviderId::ALL
        .into_iter()
        .filter(|one| match one {
            ProviderId::Github => config.github.is_some(),
            ProviderId::Notion => config.notion.is_some(),
        })
        .collect()
}

fn built((github, notion): &SourceKey) -> (Vec<Source>, Vec<groove_types::Error>) {
    let made = github
        .clone()
        .map(|one| Github::new(one).map(Source::Github));
    let made = made.into_iter().chain(
        notion
            .clone()
            .map(|one| Notion::new(one).map(Source::Notion)),
    );
    let (mut sources, mut broken) = (Vec::new(), Vec::new());
    for one in made {
        match one {
            Ok(source) => sources.push(source),
            Err(e) => broken.push(e.into()),
        }
    }
    (sources, broken)
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

/// The template one source holds, or nothing when it holds none.
pub async fn template(sources: &[Source], which: Option<ProviderId>) -> Result<Option<String>> {
    let Some(source) = pick(sources, which) else {
        return Err(unpicked(sources, which));
    };
    Ok(source.template().await?)
}

/// Why no source answers to `which`.
pub(crate) fn unpicked(sources: &[Source], which: Option<ProviderId>) -> groove_types::Error {
    let named: Vec<&str> = sources.iter().map(|one| one.id().as_str()).collect();
    groove_types::Error::invalid(match (which, named.len()) {
        (_, 0) => "no source is set up".to_string(),
        (Some(id), _) => format!(
            "{} is not set up; set up: {}",
            id.as_str(),
            named.join(", ")
        ),
        (None, _) => format!(
            "several sources are set up; name the provider: {}",
            named.join(", ")
        ),
    })
}

/// The source named, or the only one there is.
fn pick(sources: &[Source], which: Option<ProviderId>) -> Option<&Source> {
    match which {
        Some(id) => sources.iter().find(|one| one.id() == id),
        None => sources.first().filter(|_| sources.len() == 1),
    }
}

/// A Notion source, once its token reads the database.
pub async fn connect_notion(
    token: &str,
    database_id: &str,
    user_id: &str,
) -> Result<groove_types::NotionConfig> {
    Ok(Notion::connect(token, database_id, user_id).await?)
}

/// The properties one source holds, for its names to be mapped onto.
pub async fn schema(sources: &[Source], which: ProviderId) -> Result<Vec<groove_types::Property>> {
    let Some(source) = sources.iter().find(|one| one.id() == which) else {
        return Err(groove_types::Error::invalid(format!(
            "{} is not on",
            which.label()
        )));
    };
    Ok(source.schema().await?)
}

/// A GitHub source, once the host answers the token `gh` holds for it.
pub async fn connect_github(host: &str) -> Result<groove_types::GithubConfig> {
    Ok(Github::connect(host).await?)
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
