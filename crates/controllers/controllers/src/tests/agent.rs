use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

use groove_session_service::Open;
use groove_types::{AgentStatus, Session, SessionId, SessionKind, SessionState, Timestamp};

use crate::agent::Command;
use crate::{AppState, Env, SyncSpawner, dispatch};

const FAKE_CLAUDE: &str =
    "#!/bin/sh\nprintf 'ready %s\\n' \"$1\"\nread line\nprintf 'got %s\\n' \"$line\"\nexit 7\n";

fn explorer() -> Session {
    Session {
        id: SessionId::new("explorer-1"),
        title: "try".into(),
        kind: SessionKind::Explorer,
        created_at: Timestamp::new(0),
    }
}

/// An `AppState` with a fake `claude` under `<home>/.local/bin` and one open explorer.
fn state(home: &Path) -> AppState {
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).unwrap();
    let claude = bin.join("claude");
    std::fs::write(&claude, FAKE_CLAUDE).unwrap();
    std::fs::set_permissions(&claude, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut state = AppState::new(Env {
        home: home.to_path_buf(),
        data_dir: home.join("data"),
        plugin_dirs: vec![],
    });
    state.session.open.push(Open {
        session: explorer(),
        state: SessionState::default(),
        worktrees: vec![],
        delivery: vec![],
    });
    state
}

fn until(spawner: &SyncSpawner, state: &mut AppState, mut done: impl FnMut(&AppState) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        spawner.drain(state);
        if done(state) {
            return;
        }
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn agent_cmd(command: Command) -> crate::Command {
    crate::Command::Agent(command)
}

#[test]
fn start_send_and_exit_travel_through_the_loop() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let mut state = state(home.path());
    let id = SessionId::new("explorer-1");

    dispatch(
        agent_cmd(Command::Start {
            session: id.clone(),
            cwd: home.path().to_path_buf(),
            cols: 40,
            rows: 6,
        }),
        &mut state,
        &spawner,
    );
    until(&spawner, &mut state, |s| s.agent.agent(&id).is_some());
    let agent = state.agent.agent(&id).unwrap();
    assert_eq!(agent.activity.status, AgentStatus::Idle);
    assert_eq!(agent.terminal.as_ref().unwrap().size(), (40, 6));

    until(&spawner, &mut state, |s| {
        s.agent
            .agent(&id)
            .unwrap()
            .terminal
            .as_ref()
            .unwrap()
            .screen()
            .line(0)
            .starts_with("ready")
    });
    dispatch(
        agent_cmd(Command::Send {
            session: id.clone(),
            bytes: b"hi\n".to_vec(),
        }),
        &mut state,
        &spawner,
    );
    until(&spawner, &mut state, |s| {
        matches!(
            s.agent.activity(&id).unwrap().status,
            AgentStatus::Exited { .. }
        )
    });

    let agent = state.agent.agent(&id).unwrap();
    assert_eq!(agent.activity.status, AgentStatus::Exited { code: 7 });
    let screen = agent.terminal.as_ref().unwrap().screen();
    assert_eq!(screen.line(0), "ready --session-id");
    assert_eq!(screen.line(2), "got hi");
    assert!(
        home.path()
            .join("data/agent-launch/explorer-1.prompt.md")
            .is_file()
    );
}

#[test]
fn resize_reaches_the_terminal_and_end_forgets_it() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let mut state = state(home.path());
    let id = SessionId::new("explorer-1");
    dispatch(
        agent_cmd(Command::Start {
            session: id.clone(),
            cwd: home.path().to_path_buf(),
            cols: 40,
            rows: 6,
        }),
        &mut state,
        &spawner,
    );
    until(&spawner, &mut state, |s| s.agent.agent(&id).is_some());
    dispatch(
        agent_cmd(Command::Resize {
            session: id.clone(),
            cols: 30,
            rows: 8,
        }),
        &mut state,
        &spawner,
    );
    assert_eq!(
        state
            .agent
            .agent(&id)
            .unwrap()
            .terminal
            .as_ref()
            .unwrap()
            .size(),
        (30, 8)
    );
    dispatch(
        agent_cmd(Command::End {
            session: id.clone(),
        }),
        &mut state,
        &spawner,
    );
    assert!(state.agent.agent(&id).is_none());
}

#[test]
fn a_start_for_an_unknown_session_does_nothing() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let mut state = state(home.path());
    let id = SessionId::new("nope");
    dispatch(
        agent_cmd(Command::Start {
            session: id.clone(),
            cwd: home.path().to_path_buf(),
            cols: 40,
            rows: 6,
        }),
        &mut state,
        &spawner,
    );
    spawner.drain(&mut state);
    assert!(state.agent.agent(&id).is_none());
}
