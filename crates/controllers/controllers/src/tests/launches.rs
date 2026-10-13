//! A session's launch files and its start: a start asked while they are written runs once, after.

use groove_session_service::Open;
use groove_types::{Session, SessionId, SessionKind, SessionState, Timestamp};

use crate::tests::fixture::{self, until};
use crate::{AppState, Services, SyncSpawner, dispatch};

fn explorer() -> SessionId {
    SessionId::new("explorer-1")
}

/// The fixture with one open explorer and no agent.
fn opened() -> (tempfile::TempDir, SyncSpawner, Services, AppState) {
    let (home, spawner, services, mut state) = fixture::fresh();
    state.session.open.push(Open {
        session: Session {
            id: explorer(),
            title: "try".into(),
            kind: SessionKind::Explorer,
            created_at: Timestamp::new(0),
        },
        state: SessionState::default(),
        repos: vec![],
        worktrees: vec![],
        status: Default::default(),
        read: Default::default(),
        clusters: vec![],
    });
    (home, spawner, services, state)
}

fn start(state: &mut AppState, services: &Services, spawner: &SyncSpawner) {
    let start = crate::agent::Command::Start {
        session: explorer(),
        cols: 40,
        rows: 6,
    };
    dispatch(crate::Command::Agent(start), state, services, spawner);
}

/// The start held until the write lands, then launched once.
fn launched_once(state: &mut AppState, services: &Services, spawner: &SyncSpawner) {
    until(spawner, services, state, |s| {
        s.agent.agent(&explorer()).is_some()
    });
    let agent = state.agent.agent(&explorer()).expect("the agent");
    assert_eq!(agent.launch, 1, "one launch, after the write");
}

/// What the forget does once its write lands: the start it held, asked again.
fn released(
    state: &mut AppState,
    spawner: &dyn crate::Spawner,
    held: Option<crate::agent_service::Held>,
) {
    if let Some((size, prompt)) = held {
        crate::agent::asking(state, spawner, explorer(), size, prompt);
    }
}

#[test]
fn a_write_that_panics_still_lets_the_start_it_held_run() {
    let (_home, spawner, services, mut state) = opened();
    let write = async { panic!("the write broke") };
    crate::agent::rewrite(&mut state, &spawner, explorer(), write, released);
    start(&mut state, &services, &spawner);
    launched_once(&mut state, &services, &spawner);
    assert_eq!(state.errors.len(), 1, "the broken write is said");
}

#[test]
fn a_forget_asked_while_a_launch_writes_its_files_runs_after_it() {
    let (home, spawner, services, mut state) = opened();
    let held = fixture::Held::over(&spawner);
    crate::agent::start(&mut state, &held, explorer(), (40, 6));
    crate::agent::forget(&mut state, &held, &explorer());
    state.session.close(&explorer());
    held.backwards(&mut state, &services);
    let prompt = home.path().join("data/agent-launch/explorer-1.prompt.md");
    assert!(!prompt.exists(), "no launch file outlives the forget");
}

#[test]
fn a_start_asked_while_a_forget_is_out_launches_once_after_it() {
    let (home, spawner, services, mut state) = opened();
    crate::agent::forget(&mut state, &spawner, &explorer());
    start(&mut state, &services, &spawner);
    let prompt = home.path().join("data/agent-launch/explorer-1.prompt.md");
    assert!(!prompt.exists(), "no launch while the forget is out");
    launched_once(&mut state, &services, &spawner);
    assert!(prompt.exists(), "the launch wrote its files after");
}

#[test]
fn a_write_that_fails_still_lets_the_start_it_held_run() {
    let (home, spawner, services, mut state) = opened();
    let dir = home.path().join("data/agent-launch");
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    std::fs::write(&dir, "not a directory").unwrap();
    let write = async move { groove_agent_service::fresh(&dir, "explorer-1").await };
    crate::agent::rewrite(&mut state, &spawner, explorer(), write, released);
    start(&mut state, &services, &spawner);
    launched_once(&mut state, &services, &spawner);
    assert!(!state.errors.is_empty(), "the failed write is said");
}
