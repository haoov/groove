//! The `task` controller: one function per user action on the `task` service.

use groove_task_service::{fetch, list, sources};
use groove_types::TaskKey;

use crate::{AppState, Continuation, Services, Spawner};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `task.load`: every task the configured sources hold.
    Load,
    /// `task.sync`: one task, read again from its source.
    Sync { key: TaskKey },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Load => "task.load",
            Command::Sync { .. } => "task.sync",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    _services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Load => load(state, spawner),
        Command::Sync { key } => sync(state, spawner, key),
    }
}

/// Reads every source in a job; the continuation puts the list in the slice.
pub fn load(state: &mut AppState, spawner: &dyn Spawner) {
    if state.task.reading {
        return;
    }
    let sources = sources(state.config.config.as_ref());
    if sources.is_empty() {
        return;
    }
    state.task.reading = true;
    let job = state.begin("reading the tasks");
    spawner.spawn(Box::pin(async move {
        let read = list(&sources).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            state.task.reading = false;
            match read {
                Ok(tasks) => state.task.loaded(tasks),
                Err(e) => state.errors.push(e),
            }
        }) as Continuation
    }));
}

/// One task read again, for the row that shows it.
fn sync(state: &mut AppState, spawner: &dyn Spawner, key: TaskKey) {
    let sources = sources(state.config.config.as_ref());
    let job = state.begin(format!("reading {}", key.external_id()));
    spawner.spawn(Box::pin(async move {
        let read = fetch(&sources, &key).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            match read {
                Ok(task) => state.task.synced(task),
                Err(e) => state.errors.push(e),
            }
        }) as Continuation
    }));
}
