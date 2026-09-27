//! The manual section's terminals: where they open, and that they go with their session.

use crate::session::Command as SessionCommand;
use crate::shell::Command;
use crate::tests::fixture::until;
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

/// An explorer open, and a terminal opened in it.
fn opened() -> (
    tempfile::TempDir,
    SyncSpawner,
    Services,
    AppState,
    groove_types::SessionId,
    u64,
) {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let explorer = SessionCommand::OpenExplorer { title: None };
    dispatch(Cmd::Session(explorer), &mut state, &services, &spawner);
    let session = state.session.selected.clone().unwrap();
    let open = Command::Open {
        session: session.clone(),
        cols: 80,
        rows: 24,
    };
    dispatch(Cmd::Shell(open), &mut state, &services, &spawner);
    let id = state
        .shell
        .shells(&session)
        .and_then(|one| one.focused())
        .unwrap();
    let ready = session.clone();
    until(&spawner, &services, &mut state, |s| {
        s.shell.terminal(&ready, id).is_some()
    });
    (home, spawner, services, state, session, id)
}

#[test]
fn a_terminal_opens_in_the_session_s_own_directory() {
    let (home, spawner, services, mut state, session, id) = opened();
    let send = Command::Send {
        session: session.clone(),
        id,
        bytes: b"pwd; exit\n".to_vec(),
    };
    dispatch(Cmd::Shell(send), &mut state, &services, &spawner);
    let gone = session.clone();
    until(&spawner, &services, &mut state, |s| {
        s.shell
            .shells(&gone)
            .and_then(|one| one.get(id))
            .is_some_and(|one| one.exited.is_some())
    });
    let screen = state.shell.terminal(&session, id).unwrap().screen();
    let dir = home.path().join("code/worktrees").join(session.as_str());
    let said: Vec<String> = (0..screen.rows).map(|at| screen.line(at)).collect();
    assert!(
        said.iter()
            .any(|line| line.trim().ends_with(dir.to_string_lossy().as_ref())),
        "{said:?}"
    );
}

#[test]
fn closing_the_session_ends_its_terminals() {
    let (_home, spawner, services, mut state, session, _) = opened();
    let close = SessionCommand::Close {
        session: session.clone(),
    };
    dispatch(Cmd::Session(close), &mut state, &services, &spawner);
    assert!(state.shell.shells(&session).is_none());
}
