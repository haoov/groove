use std::path::PathBuf;

/// The machine the app runs on: directories the controllers need.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Env {
    pub home: PathBuf,
    /// `~/.config/groove`
    pub config_dir: PathBuf,
    /// `~/.local/share/groove`
    pub data_dir: PathBuf,
    pub plugin_dirs: Vec<PathBuf>,
}

/// The sum of the services' slices, owned on the main thread.
#[derive(Debug, Default)]
pub struct AppState {
    pub env: Env,
    pub focused: bool,
    /// What a job could not do, newest last; the feed shows them.
    pub errors: Vec<groove_types::Error>,
    /// What a job wants the user to hear, newest last.
    pub notes: Vec<String>,
    /// Jobs the user is waiting on, oldest first.
    pub pending: Vec<Pending>,
    next_pending: u64,
    pub task: groove_task_service::State,
    pub session: groove_session_service::State,
    pub workspace: groove_workspace_service::State,
    pub agent: groove_agent_service::State,
    pub config: groove_config_service::State,
}

/// One job in flight, as the status line names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub id: u64,
    pub label: String,
}

impl AppState {
    pub fn new(env: Env) -> Self {
        Self {
            env,
            ..Self::default()
        }
    }

    /// Marks a job the user waits on; `end` with the id when it lands.
    pub fn begin(&mut self, label: impl Into<String>) -> u64 {
        self.next_pending += 1;
        let id = self.next_pending;
        self.pending.push(Pending {
            id,
            label: label.into(),
        });
        id
    }

    pub fn end(&mut self, id: u64) {
        self.pending.retain(|p| p.id != id);
    }
}
