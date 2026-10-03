//! An agent launched for its session: the command line, its paths, and what its terminal reports.

use groove_agent_service::{Event as AgentEvent, Hooks, LaunchPaths, launch, palette};
use groove_types::{Session, SessionId, Timestamp};

use super::skills;
use crate::spawn::coalesced;
use crate::{AppState, Continuation, Event, Services, Spawner, apply};

/// The agent launched in a job at the worktree root; the continuation stores it or the error.
pub fn start(state: &mut AppState, spawner: &dyn Spawner, id: SessionId, size: (u16, u16)) {
    asking(state, spawner, id, size, None);
}

/// The session's agent launched with `prompt` as its first message.
pub(crate) fn asking(
    state: &mut AppState,
    spawner: &dyn Spawner,
    id: SessionId,
    size: (u16, u16),
    prompt: Option<String>,
) {
    let Some(session) = state.session.get(&id).map(|o| o.session.clone()) else {
        return;
    };
    let paths = paths(state);
    let cwd = state.config.worktree_root(&state.env.home);
    let palette = palette(state.config.theme());
    let sink = spawner.sink();
    spawner.spawn(Box::pin(async move {
        let result = launch(
            &session,
            &paths,
            &cwd,
            size,
            palette,
            Hooks {
                on_damage: on_damage(&sink, &session),
                on_exit: on_exit(&sink, &session),
            },
            prompt.as_deref(),
        );
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            let on = state
                .session
                .get(&session.id)
                .is_some_and(|open| open.state.auto_approve);
            state
                .agent
                .started(session.id.clone(), result, Timestamp::now());
            state.agent.auto_approve(&session.id, on);
        }) as Continuation
    }));
}

fn paths(state: &AppState) -> LaunchPaths {
    LaunchPaths {
        home: state.env.home.clone(),
        launch_dir: launch_dir(state),
        plugin_dirs: groove_agent_service::skills::plugin_dirs(&skills::dirs(state)),
        knowledge: knowledge(state),
        hooks: state.env.hooks.clone(),
        tools: state.env.tools.clone(),
    }
}

pub(crate) fn launch_dir(state: &AppState) -> std::path::PathBuf {
    state.env.data_dir.join("agent-launch")
}

/// The shared repo's `knowledge/`, when its copy holds one.
fn knowledge(state: &AppState) -> Option<std::path::PathBuf> {
    let dir = state.agent.shared.as_ref()?.path.join("knowledge");
    dir.is_dir().then_some(dir)
}

/// One `Damaged` in flight at most, however fast the child writes.
fn on_damage(
    sink: &std::sync::Arc<dyn crate::Deliver>,
    session: &Session,
) -> Box<dyn Fn() + Send + Sync> {
    let id = session.id.clone();
    Box::new(coalesced(sink.clone(), move || {
        let id = id.clone();
        Box::new(move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
            apply(Event::Agent(AgentEvent::Damaged { session: id }), state)
        })
    }))
}

fn on_exit(
    sink: &std::sync::Arc<dyn crate::Deliver>,
    session: &Session,
) -> Box<dyn FnOnce(u32) + Send> {
    let sink = sink.clone();
    let id = session.id.clone();
    Box::new(move |code| {
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                let event = AgentEvent::Exited {
                    session: id,
                    code,
                    at: Timestamp::now(),
                };
                apply(Event::Agent(event), state);
            },
        ));
    })
}
