//! The `session` controller: one function per user action on the `session` service.

use groove_types::{Error, SessionId, Timestamp};

use crate::{AppState, Continuation, Services, Spawner, agent};

/// The grid an agent starts on; the pane resizes it on its first frame.
const FIRST_SIZE: (u16, u16) = (80, 24);

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `session.restore`: the rail as it was when the app last closed, agents started.
    Restore,
    /// `session.open_explorer`: a session with no ticket yet, its agent started.
    OpenExplorer { title: Option<String> },
    /// `session.rename_explorer`
    RenameExplorer { session: SessionId, title: String },
    /// `session.discard_explorer`: end the agent, delete the session and what it owns.
    DiscardExplorer { session: SessionId },
    /// `session.select`: make it the current one.
    Select { session: SessionId },
    /// `session.close`: end the agent, drop the row; the session stays on disk.
    Close { session: SessionId },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::Restore => "session.restore",
            Command::OpenExplorer { .. } => "session.open_explorer",
            Command::RenameExplorer { .. } => "session.rename_explorer",
            Command::DiscardExplorer { .. } => "session.discard_explorer",
            Command::Select { .. } => "session.select",
            Command::Close { .. } => "session.close",
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
        Command::Restore => restore(services, spawner),
        Command::OpenExplorer { title } => {
            open_explorer(state, services, spawner, title.as_deref())
        }
        Command::RenameExplorer { session, title } => {
            rename_explorer(state, services, spawner, &session, &title)
        }
        Command::DiscardExplorer { session } => {
            discard_explorer(state, services, spawner, &session)
        }
        Command::Select { session } => select(state, services, spawner, &session),
        Command::Close { session } => close(state, services, spawner, &session),
    }
}

/// Reads the opened rows; the continuation puts them on the rail and starts each agent.
pub fn restore(services: &Services, spawner: &dyn Spawner) {
    let service = services.session.clone();
    spawner.spawn(Box::pin(async move {
        let result = service.opened().await;
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                let rows = match result {
                    Ok(rows) => rows,
                    Err(e) => return state.errors.push(e),
                };
                let last_seen = rows
                    .iter()
                    .max_by_key(|(_, st)| st.seen_at)
                    .map(|(s, _)| s.id.clone());
                for (session, session_state) in rows {
                    let id = session.id.clone();
                    state.session.restore(session, session_state);
                    agent::start(state, spawner, id, FIRST_SIZE);
                }
                state.session.selected = last_seen;
            },
        ) as Continuation
    }));
}

pub fn open_explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    title: Option<&str>,
) {
    let now = Timestamp::now();
    let session = groove_session_service::explorer(title, now);
    let id = session.id.clone();
    state.session.open(session.clone(), now);
    agent::start(state, spawner, id, FIRST_SIZE);
    let service = services.session.clone();
    record(spawner, async move {
        service.create_explorer(&session, now).await
    });
}

pub fn rename_explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
    title: &str,
) {
    state.session.rename(id, title);
    let (service, id, title) = (services.session.clone(), id.clone(), title.to_string());
    record(spawner, async move {
        service.rename_explorer(&id, &title).await
    });
}

pub fn discard_explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &dyn Spawner,
    id: &SessionId,
) {
    agent::end(state, id);
    state.session.close(id);
    let (service, id) = (services.session.clone(), id.clone());
    record(spawner, async move { service.remove(&id).await });
}

pub fn select(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    let now = Timestamp::now();
    state.session.select(id, now);
    let (service, id) = (services.session.clone(), id.clone());
    record(spawner, async move { service.set_seen(&id, now).await });
}

pub fn close(state: &mut AppState, services: &Services, spawner: &dyn Spawner, id: &SessionId) {
    agent::end(state, id);
    state.session.close(id);
    let (service, id) = (services.session.clone(), id.clone());
    record(spawner, async move { service.set_opened(&id, None).await });
}

/// A write whose only result is success or an error for the feed.
fn record(spawner: &dyn Spawner, write: impl Future<Output = Result<(), Error>> + Send + 'static) {
    spawner.spawn(Box::pin(async move {
        let result = write.await;
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            if let Err(e) = result {
                state.errors.push(e);
            }
        }) as Continuation
    }));
}
