use crate::{AppState, Spawner, agent, config, session, task, workspace};

/// One variant per controller function, grouped by controller.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Task(task::Command),
    Session(session::Command),
    Workspace(workspace::Command),
    Agent(agent::Command),
    Config(config::Command),
}

impl Command {
    /// `controller.function`: the palette entry, the tool name, the timeline label.
    pub fn id(&self) -> &'static str {
        match self {
            Command::Task(c) => c.id(),
            Command::Session(c) => c.id(),
            Command::Workspace(c) => c.id(),
            Command::Agent(c) => c.id(),
            Command::Config(c) => c.id(),
        }
    }
}

/// The one match. The sync part runs now; the async part goes through the spawner.
pub fn dispatch(command: Command, state: &mut AppState, spawner: &dyn Spawner) {
    match command {
        Command::Task(c) => task::dispatch(c, state, spawner),
        Command::Session(c) => session::dispatch(c, state, spawner),
        Command::Workspace(c) => workspace::dispatch(c, state, spawner),
        Command::Agent(c) => agent::dispatch(c, state, spawner),
        Command::Config(c) => config::dispatch(c, state, spawner),
    }
}
