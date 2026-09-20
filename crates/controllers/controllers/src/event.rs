use crate::AppState;

/// From winit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    Focus(bool),
}

/// External inputs with no originating command.
#[derive(Debug)]
pub enum Event {
    Task(groove_task_service::Event),
    Session(groove_session_service::Event),
    Workspace(groove_workspace_service::Event),
    Agent(groove_agent_service::Event),
    Config(groove_config_service::Event),
    Window(Window),
}

/// One match, one line per variant, each delegating to the owning service.
pub fn apply(event: Event, state: &mut AppState) {
    match event {
        Event::Task(e) => groove_task_service::apply(&mut state.task, e),
        Event::Session(e) => groove_session_service::apply(&mut state.session, e),
        Event::Workspace(e) => groove_workspace_service::apply(&mut state.workspace, e),
        Event::Agent(e) => groove_agent_service::apply(&mut state.agent, e),
        Event::Config(e) => groove_config_service::apply(&mut state.config, e),
        Event::Window(Window::Focus(focused)) => {
            state.focused = focused;
            if focused {
                state.workspace.poll.woke();
            }
        }
    }
}
