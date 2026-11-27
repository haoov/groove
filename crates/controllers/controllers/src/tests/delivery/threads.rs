//! The MR's threads the surface lists.

use groove_types::{MrNote, MrThread, NotePosition, Timestamp};

use super::*;
use crate::Services;

/// The fixture with a worktree.
fn ready() -> (tempfile::TempDir, SyncSpawner, crate::AppState, Services) {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    worktree(&mut state, &services, &spawner);
    (home, spawner, state, services)
}

/// What the threads open with, as the list reads them.
fn said(state: &crate::AppState) -> Vec<String> {
    state
        .delivery
        .shown
        .iter()
        .filter_map(|note| note.opening())
        .map(|said| said.body.clone())
        .collect()
}

#[test]
fn a_thread_the_forge_answered_joins_the_list_without_a_read() {
    let (_home, _spawner, mut state, _services) = ready();
    took(&mut state);
    crate::delivery::show(&mut state);
    assert_eq!(said(&state), vec!["reviewer said so".to_string()]);
}

#[test]
fn the_surface_shows_the_threads_of_the_worktree_selected_now() {
    let (_home, spawner, mut state, _services) = ready();
    took(&mut state);
    let selected = state.session.selected.take();
    crate::workspace::follow(&mut state, &spawner);
    assert!(said(&state).is_empty(), "no worktree, no threads");
    state.session.selected = selected;
    crate::workspace::follow(&mut state, &spawner);
    assert_eq!(said(&state), ["reviewer said so"]);
}

/// The selected worktree's MR, as a read of the forge answers it.
fn took(state: &mut crate::AppState) {
    let worktree = state
        .session
        .selected_worktree()
        .expect("a worktree")
        .id
        .clone();
    let mr = groove_types::Mr {
        id: groove_types::MrId::new("m1"),
        worktree: worktree.clone(),
        forge: groove_types::Forge::Github,
        remote_id: "7".into(),
        url: "https://example.test/pull/7".into(),
        state: groove_types::MrState::Open,
    };
    let read = snapshot();
    state.delivery.took(
        &worktree,
        groove_delivery_service::Delivered {
            mr,
            read,
            unassigned: None,
        },
        groove_types::Timestamp::now(),
    );
}

/// What a read of the MR brings back: one thread on a later line.
fn snapshot() -> groove_delivery_service::Snapshot {
    groove_delivery_service::Snapshot {
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
