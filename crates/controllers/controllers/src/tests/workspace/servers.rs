//! The open file's language servers through the controller, on a server scripted in `sh`.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

use groove_types::{LanguageServer, Severity, WorktreeId};

use super::*;
use crate::AppState;

/// Answers `initialize`, then one diagnostic on columns 4 to 8 of each file opened. A
/// definition lands on line 3, column 5 of the same file, or outside once `.outside` exists.
const FAKE_SERVER: &str = r#"#!/bin/sh
send() { printf 'Content-Length: %s\r\n\r\n%s' "${#1}" "$1"; }
next() {
  len=0
  while IFS= read -r line; do
    line=$(printf %s "$line" | tr -d '\r')
    [ -z "$line" ] && break
    case "$line" in Content-Length:*) len=${line#Content-Length: } ;; esac
  done
  [ "$len" -gt 0 ] || exit 0
  body=$(dd bs=1 count="$len" 2>/dev/null)
}
next
id=$(printf %s "$body" | sed 's/.*"id":\([0-9]*\).*/\1/')
send "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"capabilities\":{}}}"
while next; do
  case "$body" in
    *textDocument/didOpen*)
      uri=$(printf %s "$body" | sed 's/.*"uri":"\([^"]*\)".*/\1/')
      range='{"start":{"line":0,"character":4},"end":{"line":0,"character":8}}'
      send "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{\"uri\":\"$uri\",\"diagnostics\":[{\"range\":$range,\"severity\":1,\"message\":\"fake\"}]}}"
      ;;
    *textDocument/definition*)
      id=$(printf %s "$body" | sed 's/.*"id":\([0-9]*\).*/\1/')
      uri=$(printf %s "$body" | sed 's/.*"uri":"\([^"]*\)".*/\1/')
      [ -f .outside ] && uri=file:///elsewhere/lib.py
      range='{"start":{"line":2,"character":4},"end":{"line":2,"character":5}}'
      send "{\"jsonrpc\":\"2.0\",\"id\":$id,\"result\":{\"uri\":\"$uri\",\"range\":$range}}"
      ;;
  esac
done
"#;

/// The config's one server for Python is `program`.
fn serving_python(state: &mut AppState, program: &str) {
    let server = LanguageServer {
        name: "fake".into(),
        languages: vec!["python".into()],
        program: program.into(),
        args: vec![],
        unset: vec!["VIRTUAL_ENV".into()],
        settings: serde_json::Value::Null,
        off: false,
    };
    if let Some(config) = state.config.config.as_mut() {
        config.language_servers = Some(vec![server]);
    }
}

fn fake_server(home: &Path) -> String {
    let path = home.join("fake-server");
    std::fs::write(&path, FAKE_SERVER).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.to_string_lossy().into_owned()
}

/// The worktree with `a.py` open, and its id.
fn python_open(state: &mut AppState, services: &Services, spawner: &SyncSpawner) -> WorktreeId {
    let dir = worktree(state, services, spawner);
    std::fs::write(Path::new(&dir).join("a.py"), "x = None\n\ny = x\n").unwrap();
    until(spawner, services, state, |s| !s.workspace.files.is_empty());
    let open = workspace::Command::OpenFile {
        at: None,
        path: "a.py".into(),
    };
    dispatch(Cmd::Workspace(open), state, services, spawner);
    until(spawner, services, state, |s| s.workspace.active().is_some());
    state.workspace.worktree.clone().expect("a worktree")
}

/// Like `until`, but lets the detached readers run between drains.
fn settled(
    spawner: &SyncSpawner,
    services: &Services,
    state: &mut AppState,
    done: impl Fn(&AppState) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done(state) {
        assert!(Instant::now() < deadline, "timed out: {:?}", state.errors);
        spawner.settle(Duration::from_millis(20), state, services);
    }
}

#[test]
fn a_python_file_opened_shows_what_its_server_finds() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let program = fake_server(home.path());
    serving_python(&mut state, &program);
    let worktree = python_open(&mut state, &services, &spawner);
    settled(&spawner, &services, &mut state, |s| {
        !s.workspace
            .servers
            .diagnostics(&worktree, "a.py")
            .is_empty()
    });
    let found = state.workspace.servers.diagnostics(&worktree, "a.py");
    assert_eq!(found[0].message, "fake");
    assert_eq!(found[0].severity, Severity::Error);
    assert_eq!(
        (found[0].start, found[0].end),
        (Caret::new(0, 4), Caret::new(0, 8))
    );
}

#[test]
fn a_server_not_installed_is_said_once_and_left_down() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    serving_python(&mut state, "groove-no-such-server");
    python_open(&mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| !s.errors.is_empty());
    let said = &state.errors[0];
    assert!(
        said.what
            .message
            .starts_with("fake is not installed: `groove-no-such-server` is not on the login PATH"),
        "{:?}",
        said
    );
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Edit(Edit::Insert("y".into())),
    );
    spawner.drain(&mut state, &services);
    assert_eq!(state.errors.len(), 1, "a keystroke starts nothing");
}

#[test]
fn a_file_no_server_serves_starts_none() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    serving_python(&mut state, "groove-no-such-server");
    editing(&mut state, &services, &spawner);
    spawner.drain(&mut state, &services);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

/// `a.py` open on the fake server, up and having said what it found.
fn served(home: &Path, state: &mut AppState, services: &Services, spawner: &SyncSpawner) {
    let program = fake_server(home);
    serving_python(state, &program);
    let worktree = python_open(state, services, spawner);
    settled(spawner, services, state, |s| {
        !s.workspace
            .servers
            .diagnostics(&worktree, "a.py")
            .is_empty()
    });
}

fn define(state: &mut AppState, services: &Services, spawner: &SyncSpawner) {
    let asked = workspace::Command::Definition {
        path: "a.py".into(),
        at: Caret::new(0, 0),
    };
    dispatch(Cmd::Workspace(asked), state, services, spawner);
}

#[test]
fn a_definition_in_the_worktree_opens_with_the_caret_on_it() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    served(home.path(), &mut state, &services, &spawner);
    define(&mut state, &services, &spawner);
    settled(&spawner, &services, &mut state, |s| {
        s.workspace
            .active()
            .is_some_and(|open| open.new.caret() == Caret::new(2, 4))
    });
    assert_eq!(
        state.workspace.active().map(|open| open.path.as_str()),
        Some("a.py")
    );
    let landed = state.workspace.servers.landed.clone();
    assert_eq!(
        landed,
        Some(("a.py".into(), Caret::new(2, 4))),
        "for the view to centre on"
    );
}

#[test]
fn a_definition_outside_the_worktree_is_said_at_the_spot_and_opens_nothing() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    served(home.path(), &mut state, &services, &spawner);
    let dir = state
        .session
        .selected_worktree()
        .map(|one| one.dir())
        .expect("a worktree");
    std::fs::write(dir.join(".outside"), "").unwrap();
    define(&mut state, &services, &spawner);
    let told = |s: &AppState| {
        s.workspace
            .servers
            .hovered
            .as_ref()
            .filter(|one| one.told)
            .cloned()
    };
    settled(&spawner, &services, &mut state, |s| told(s).is_some());
    let said = told(&state).and_then(|one| one.said).unwrap_or_default();
    assert!(
        said.contains("outside the worktree, in /elsewhere/lib.py"),
        "{said}"
    );
    assert!(state.notes.is_empty(), "said at the spot, not in the feed");
    let caret = state.workspace.active().map(|open| open.new.caret());
    assert_eq!(caret, Some(Caret::new(0, 0)), "the caret stays");
}
