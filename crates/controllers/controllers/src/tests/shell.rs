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
    // A long path wraps, so the rows are read as one run of text.
    let said: String = (0..screen.rows).map(|at| screen.line(at)).collect();
    assert!(said.contains(dir.to_string_lossy().as_ref()), "{said:?}");
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

#[test]
fn a_word_selected_on_a_terminal_copies_to_the_clipboard() {
    let (_home, spawner, services, mut state, session, id) = opened();
    let send = Command::Send {
        session: session.clone(),
        id,
        bytes: b"printf 'gro''ovy\\n'\n".to_vec(),
    };
    dispatch(Cmd::Shell(send), &mut state, &services, &spawner);
    let shown = |s: &AppState| {
        let screen = s.shell.terminal(&session, id)?.screen();
        (0..screen.rows).find_map(|at| {
            let line = screen.line(at);
            let output = line.trim_end().ends_with("groovy") && !line.contains("printf");
            line.find("groovy").filter(|_| output).map(|col| (col, at))
        })
    };
    until(&spawner, &services, &mut state, |s| shown(s).is_some());
    let (col, row) = shown(&state).unwrap();
    let select = Command::Select {
        session: session.clone(),
        id,
        col: col + 2,
        row,
        kind: crate::agent_service::Select::Word,
        from: true,
    };
    dispatch(Cmd::Shell(select), &mut state, &services, &spawner);
    let copy = Command::Copy {
        session: session.clone(),
        id,
    };
    dispatch(Cmd::Shell(copy), &mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    let copied = spawner.block_on(services.clipboard.read());
    assert_eq!(copied.as_deref(), Some("groovy"));
}
