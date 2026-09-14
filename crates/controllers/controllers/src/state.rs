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
    pub task: groove_task_service::State,
    pub session: groove_session_service::State,
    pub workspace: groove_workspace_service::State,
    pub agent: groove_agent_service::State,
    pub config: groove_config_service::State,
}

impl AppState {
    pub fn new(env: Env) -> Self {
        Self {
            env,
            ..Self::default()
        }
    }
}
