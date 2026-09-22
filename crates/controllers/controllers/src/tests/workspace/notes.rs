//! A note left on a line, written again, resolved and taken away.

use groove_types::{Anchor, AnnotationId, MrNote, MrThread, NotePosition, Timestamp};

use super::*;
use crate::workspace::NoteAct as Act;

/// The fixture with a worktree, ready to take notes.
fn ready(home: &std::path::Path, spawner: &SyncSpawner) -> (crate::AppState, Services) {
    pooled_clone(home);
    let services = services(spawner, home);
    let mut state = state(home);
    worktree(&mut state, &services, spawner);
    (state, services)
}

fn send(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
    command: workspace::Command,
) {
    dispatch(Cmd::Workspace(command), state, services, spawner);
    until(spawner, services, state, |s| s.pending.is_empty());
}

fn leave(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
    line: u32,
    content: &str,
) {
    send(
        state,
        services,
        spawner,
        workspace::Command::Note(Act::Create {
            anchor: Anchor::line("a.txt", line),
            content: content.to_string(),
            author: "rsabbah".into(),
        }),
    );
}

/// What the notes say, as the list reads them.
fn said(state: &crate::AppState) -> Vec<String> {
    state
        .workspace
        .notes
        .iter()
        .filter_map(|note| note.opening())
        .map(|said| said.body.clone())
        .collect()
}

fn first(state: &crate::AppState) -> AnnotationId {
    state
        .workspace
        .notes
        .first()
        .and_then(|note| note.id())
        .cloned()
        .expect("a note")
}

#[test]
fn a_note_left_on_a_line_reads_back_on_the_list() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services) = ready(home.path(), &spawner);

    leave(&mut state, &services, &spawner, 4, "issue: this leaks");
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert_eq!(said(&state), vec!["issue: this leaks".to_string()]);
    let note = state.workspace.notes.first().expect("a note");
    assert!(note.on("a.txt", 4));
    assert!(!note.resolved);
}

#[test]
fn the_notes_of_the_session_read_by_line() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services) = ready(home.path(), &spawner);

    leave(&mut state, &services, &spawner, 9, "second");
    leave(&mut state, &services, &spawner, 2, "first");
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::GetNotes,
    );
    assert_eq!(
        said(&state),
        vec!["first".to_string(), "second".to_string()]
    );
}

#[test]
fn new_words_replace_the_old_ones() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services) = ready(home.path(), &spawner);

    leave(&mut state, &services, &spawner, 1, "issue: this leaks");
    let id = first(&state);
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Note(Act::Update {
            id,
            content: "nitpick: name it".into(),
        }),
    );
    assert_eq!(said(&state), vec!["nitpick: name it".to_string()]);
}

#[test]
fn a_resolved_note_stays_on_the_list_as_resolved() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services) = ready(home.path(), &spawner);

    leave(&mut state, &services, &spawner, 1, "issue: this leaks");
    let id = first(&state);
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Note(Act::Resolve { id }),
    );
    let note = state.workspace.notes.first().expect("the note");
    assert!(note.resolved);
}

#[test]
fn a_deleted_note_leaves_the_list() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services) = ready(home.path(), &spawner);

    leave(&mut state, &services, &spawner, 1, "issue: this leaks");
    let id = first(&state);
    send(
        &mut state,
        &services,
        &spawner,
        workspace::Command::Note(Act::Delete { id }),
    );
    assert!(state.workspace.notes.is_empty());
}

#[test]
fn a_thread_the_forge_answered_joins_the_list_without_a_read() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let (mut state, services) = ready(home.path(), &spawner);

    leave(&mut state, &services, &spawner, 1, "issue: this leaks");
    state.workspace.delivery.read = Some(snapshot());
    state.workspace.remerge();
    assert_eq!(
        said(&state),
        vec![
            "issue: this leaks".to_string(),
            "reviewer said so".to_string()
        ]
    );
}

/// What a read of the MR brings back: one thread on a later line.
fn snapshot() -> groove_workspace_service::Snapshot {
    groove_workspace_service::Snapshot {
        head: "cafe1234".into(),
        node: "n1".into(),
        number: "7".into(),
        details: details(),
        ci: None,
        threads: vec![MrThread {
            id: "t1".into(),
            notes: vec![MrNote {
                author: "reviewer".into(),
                body: "reviewer said so".into(),
                created_at: Timestamp::new(10),
                resolved: false,
                resolvable: true,
                position: Some(NotePosition {
                    new_path: Some("a.txt".into()),
                    new_line: Some(40),
                    ..NotePosition::default()
                }),
            }],
        }],
    }
}

fn details() -> groove_types::MrDetails {
    groove_types::MrDetails {
        title: "fix: one".into(),
        description: String::new(),
        author: "haoov".into(),
        source_branch: "fix/one".into(),
        target_branch: "main".into(),
        state: groove_types::MrState::Open,
        draft: false,
        created_at: Timestamp::new(0),
        updated_at: Timestamp::new(0),
        web_url: "https://example.com/1".into(),
        approval: None,
        reviewers: vec![],
    }
}
