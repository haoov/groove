//! An agent launched for its session: the command line, its paths, and what its stream reports.

use groove_agent_service::{Event as AgentEvent, Held, LaunchPaths, Stream, StreamHooks, launch};
use groove_types::{Error, Session, SessionId, Timestamp};

use super::skills;
use crate::{AppState, Continuation, Event, Services, Spawner, apply};

/// The agent launched in a job at the worktree root; the continuation stores it or the error.
pub fn start(state: &mut AppState, spawner: &dyn Spawner, id: SessionId) {
    asking(state, spawner, id, None);
}

/// The session's agent launched with `prompt` as its first message, on the session's write line.
pub(crate) fn asking(
    state: &mut AppState,
    spawner: &dyn Spawner,
    id: SessionId,
    prompt: Option<String>,
) {
    let Some(prompt) = state.agent.hold(&id, prompt) else {
        return;
    };
    let Some(session) = state.session.get(&id).map(|o| o.session.clone()) else {
        return;
    };
    let launched = state.agent.launch(&id);
    let sink = spawner.sink();
    let one = Launch {
        paths: paths(state),
        dirs: skills::dirs(state),
        shared: shared_copy(state),
        cwd: state.config.worktree_root(&state.env.home),
        sink,
        launched,
        session,
        prompt,
    };
    let mut turn = state.agent.writing(&id);
    spawner.spawn(Box::pin(async move {
        turn.wait().await;
        let result = tokio::spawn(launched_in(one)).await.unwrap_or_else(|e| {
            let why = format!("the agent did not launch: {e}");
            Err(Error::new(groove_types::ErrorKind::Internal, why))
        });
        turn.end();
        stored((id, launched), result)
    }));
}

/// What one launch needs, read on the main thread.
struct Launch {
    session: Session,
    paths: LaunchPaths,
    dirs: groove_skill_service::skills::Dirs,
    shared: Option<std::path::PathBuf>,
    cwd: std::path::PathBuf,
    sink: std::sync::Arc<dyn crate::Deliver>,
    launched: u64,
    prompt: Option<String>,
}

/// The launch files written and the agent spawned on its stream.
async fn launched_in(one: Launch) -> Result<Stream, Error> {
    let paths = LaunchPaths {
        plugin_dirs: groove_skill_service::skills::plugin_dirs(&one.dirs).await?,
        knowledge: groove_skill_service::listing::knowledge(one.shared.as_deref()).await,
        ..one.paths
    };
    let (session, prompt) = (&one.session, one.prompt.as_deref());
    let on_exit = on_exit(&one.sink, session, one.launched);
    let on_heard = on_heard(&one.sink, session, one.launched);
    let hooks = StreamHooks { on_heard, on_exit };
    launch(session, &paths, &one.cwd, hooks, prompt).await
}

/// The launch kept on its session, or ended when the session went; a start held meanwhile runs.
fn stored((id, launched): (SessionId, u64), result: Result<Stream, Error>) -> Continuation {
    Box::new(
        move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
            let held = state.agent.written(&id);
            let now = Timestamp::now();
            let orphan = match state.session.get(&id) {
                Some(_) => state.agent.started((id.clone(), launched), result, now),
                None => result.ok(),
            };
            if let Some(orphan) = orphan {
                let _ = orphan.terminate();
            }
            if let Some(prompt) = held {
                asking(state, spawner, id, prompt);
            }
        },
    )
}

fn paths(state: &AppState) -> LaunchPaths {
    LaunchPaths {
        home: state.env.home.clone(),
        launch_dir: launch_dir(state),
        plugin_dirs: Vec::new(),
        knowledge: None,
        hooks: state.env.hooks.clone(),
        tools: state.env.tools.clone(),
    }
}

/// A write of `session`'s launch files, a panic read as an error; a start asked meanwhile waits.
pub(crate) fn rewrite(
    state: &mut AppState,
    spawner: &dyn Spawner,
    session: SessionId,
    write: impl Future<Output = Result<(), Error>> + Send + 'static,
    then: impl FnOnce(&mut AppState, &dyn Spawner, Option<Held>) + Send + 'static,
) {
    let mut turn = state.agent.writing(&session);
    spawner.spawn(Box::pin(async move {
        turn.wait().await;
        let written = tokio::spawn(write).await.unwrap_or_else(|e| {
            let why = format!("the launch files were not written: {e}");
            Err(Error::new(groove_types::ErrorKind::Internal, why))
        });
        turn.end();
        Box::new(
            move |state: &mut AppState, _: &Services, spawner: &dyn Spawner| {
                let held = state.agent.written(&session);
                if let Err(e) = written {
                    state.failed(e);
                }
                then(state, spawner, held);
            },
        ) as Continuation
    }));
}

fn shared_copy(state: &AppState) -> Option<std::path::PathBuf> {
    state.skill.shared.as_ref().map(|one| one.path.clone())
}

pub(crate) fn launch_dir(state: &AppState) -> std::path::PathBuf {
    state.env.data_dir.join("agent-launch")
}

/// What the stream says beside the transcript, applied to the launch it came from.
fn on_heard(
    sink: &std::sync::Arc<dyn crate::Deliver>,
    session: &Session,
    launch: u64,
) -> Box<dyn Fn(groove_agent_service::StreamHeard) + Send + Sync> {
    let (sink, id) = (sink.clone(), session.id.clone());
    Box::new(move |heard| {
        let session = id.clone();
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                let at = Timestamp::now();
                let event = AgentEvent::Streamed {
                    session,
                    launch,
                    heard,
                    at,
                };
                apply(Event::Agent(event), state);
            },
        ));
    })
}

fn on_exit(
    sink: &std::sync::Arc<dyn crate::Deliver>,
    session: &Session,
    launch: u64,
) -> Box<dyn FnOnce(u32) + Send> {
    let sink = sink.clone();
    let id = session.id.clone();
    Box::new(move |code| {
        sink.deliver(Box::new(
            move |state: &mut AppState, _: &Services, _: &dyn Spawner| {
                let event = AgentEvent::Exited {
                    session: id,
                    launch,
                    code,
                    at: Timestamp::now(),
                };
                apply(Event::Agent(event), state);
            },
        ));
    })
}
