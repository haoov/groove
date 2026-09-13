/// The sum of the services' slices, owned on the main thread.
#[derive(Debug, Default)]
pub struct AppState {
    pub focused: bool,
    pub task: groove_task_service::State,
    pub session: groove_session_service::State,
    pub workspace: groove_workspace_service::State,
    pub agent: groove_agent_service::State,
    pub config: groove_config_service::State,
}
