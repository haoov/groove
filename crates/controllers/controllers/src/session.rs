//! The `session` controller: one function per user action on the `session` service.

use groove_types::{SessionId, Timestamp};

use crate::{AppState, Spawner, agent};

/// The grid an agent starts on; the pane resizes it on its first frame.
const FIRST_SIZE: (u16, u16) = (80, 24);

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// `session.open_explorer`: a session with no ticket yet, its agent started.
    OpenExplorer { title: Option<String> },
    /// `session.select`: make it the current one.
    Select { session: SessionId },
    /// `session.close`: end the agent, drop the row; the session stays on disk.
    Close { session: SessionId },
}

impl Command {
    pub fn id(&self) -> &'static str {
        match self {
            Command::OpenExplorer { .. } => "session.open_explorer",
            Command::Select { .. } => "session.select",
            Command::Close { .. } => "session.close",
        }
    }
}

pub fn dispatch(command: Command, state: &mut AppState, spawner: &dyn Spawner) {
    match command {
        Command::OpenExplorer { title } => open_explorer(state, spawner, title.as_deref()),
        Command::Select { session } => state.session.select(&session, Timestamp::now()),
        Command::Close { session } => close(state, &session),
    }
}

pub fn open_explorer(state: &mut AppState, spawner: &dyn Spawner, title: Option<&str>) {
    let now = Timestamp::now();
    let session = groove_session_service::explorer(title, now);
    let id = session.id.clone();
    state.session.open(session, now);
    agent::start(state, spawner, id, FIRST_SIZE);
}

pub fn close(state: &mut AppState, session: &SessionId) {
    agent::end(state, session);
    state.session.close(session);
}
