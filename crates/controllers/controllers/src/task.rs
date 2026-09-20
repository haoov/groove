//! The `task` controller: one function per user action on the `task` service.

pub mod attention;
mod finish;
mod plan;
mod status;

pub use plan::Landing;
pub mod time;

use groove_session_service::task_session;
use groove_task_service::{fetch, list, sources};
use groove_types::{ExternalId, SessionId, SessionKind, StatusIntent, Task, TaskKey, Timestamp};

use crate::{AppState, Continuation, Services, Spawner, agent, session};

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `task.load`: every task the configured sources hold.
    Load,
    /// `task.sync`: one task, read again from its source.
    Sync { key: TaskKey },
    /// `task.open`: the session that works this task, created or selected.
    Open { short_id: String },
    /// `task.log_hours`: what the clock measured and the source has not been told.
    LogHours { external_id: ExternalId },
    /// `task.finish`: the task done at its source, and its session taken away.
    Finish { session: SessionId },
    /// `task.delete_local`: the session taken away here, the source left alone.
    DeleteLocal { session: SessionId },
    /// `task.set_status`: by lifecycle only, in progress on open and done on finish.
    SetStatus {
        external_id: ExternalId,
        intent: StatusIntent,
    },
    /// `task.plan`: one task moved above another, or to the end of its own side.
    Plan(Landing),
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Load => "task.load",
            Command::Sync { .. } => "task.sync",
            Command::Open { .. } => "task.open",
            Command::LogHours { .. } => "task.log_hours",
            Command::Finish { .. } => "task.finish",
            Command::DeleteLocal { .. } => "task.delete_local",
            Command::SetStatus { .. } => "task.set_status",
            Command::Plan { .. } => "task.plan",
        }
    }
}

pub fn dispatch(
    command: Command,
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
) {
    match command {
        Command::Load => load(state, services, spawner),
        Command::Sync { key } => sync(state, spawner, key),
        Command::Open { short_id } => open(state, services, spawner, &short_id),
        Command::LogHours { external_id } => {
            time::log_hours(state, services, spawner, &external_id)
        }
        Command::Finish { session } => finish::finish(state, services, spawner, &session),
        Command::DeleteLocal { session } => finish::locally(state, services, spawner, &session),
        Command::SetStatus {
            external_id,
            intent,
        } => status::set(state, spawner, &external_id, intent),
        Command::Plan(landing) => plan::reorder(state, services, spawner, &landing),
    }
}

/// The session that works this task: the one it already has, or a new one with its
/// agent started.
pub fn open(state: &mut AppState, services: &Services, spawner: &dyn Spawner, short_id: &str) {
    let Some(task) = state.task.get(short_id).cloned() else {
        return;
    };
    if let Some(id) = working(state, &task) {
        return session::select(state, services, spawner, &id);
    }
    let now = Timestamp::now();
    let session = task_session(&task, now);
    let id = session.id.clone();
    state.session.open(session.clone(), now);
    crate::workspace::follow(state, spawner);
    follow(state, spawner);
    status::set(state, spawner, &task.external_id, StatusIntent::InProgress);
    agent::start(state, spawner, id, session::FIRST_SIZE);
    let service = services.session.clone();
    session::listed(spawner, session::NO_PENDING, async move {
        service.create_task(&session, &task, now).await
    });
}

/// The task of the selected session, read once with its body.
pub fn follow(state: &mut AppState, spawner: &dyn Spawner) {
    let Some(open) = state.session.selected() else {
        return;
    };
    let SessionKind::Task { external_id } = &open.session.kind else {
        return;
    };
    let external_id = external_id.clone();
    let known = state
        .task
        .by_external(&external_id)
        .is_some_and(|task| state.task.body(&task.short_id).is_some());
    if known {
        return;
    }
    match TaskKey::parse(&external_id) {
        Ok(key) => sync(state, spawner, key),
        Err(e) => state.errors.push(e),
    }
}

/// The open session working this task, if one already is.
fn working(state: &AppState, task: &Task) -> Option<SessionId> {
    state
        .session
        .open
        .iter()
        .find(|open| open.session.kind.works(&task.external_id))
        .map(|open| open.session.id.clone())
}

/// Reads what the database holds of the tasks: the user's order and the hours.
fn stored(services: &Services, spawner: &dyn Spawner) {
    let service = services.task.clone();
    spawner.spawn(Box::pin(async move {
        let read = service.order().await;
        let time = service.time().await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            match read {
                Ok(order) => state.task.plan = order,
                Err(e) => state.errors.push(e),
            }
            match time {
                Ok(time) => state.task.time = time.into_iter().collect(),
                Err(e) => state.errors.push(e),
            }
        }) as Continuation
    }));
}

/// Reads the plan, then every source in a job; the continuations fill the slice.
pub fn load(state: &mut AppState, services: &Services, spawner: &dyn Spawner) {
    stored(services, spawner);
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
            attention::reread(state, Timestamp::now());
        }) as Continuation
    }));
}

/// One task read again, with its body, for the views that show it.
fn sync(state: &mut AppState, spawner: &dyn Spawner, key: TaskKey) {
    let id = key.external_id();
    if !state.task.syncing.insert(id.clone()) {
        return;
    }
    let sources = sources(state.config.config.as_ref());
    let job = state.begin(format!("reading {id}"));
    spawner.spawn(Box::pin(async move {
        let read = fetch(&sources, &key).await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            state.end(job);
            state.task.syncing.remove(&id);
            match read {
                Ok(read) => state.task.synced(read),
                Err(e) => state.errors.push(e),
            }
            attention::reread(state, Timestamp::now());
        }) as Continuation
    }));
}
