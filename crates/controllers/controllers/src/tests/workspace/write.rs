//! What a forge write refuses before it is ever sent.

use groove_types::{Forge, Mr, MrId, MrState, WorktreeId};

use super::*;

fn holding(state: &mut crate::AppState, worktree: &WorktreeId, mr: MrState) {
    state.workspace.delivery.mr = Some(Mr {
        id: MrId::new("m1"),
        worktree: worktree.clone(),
        forge: Forge::Github,
        remote_id: "7".into(),
        url: "https://example.test/pull/7".into(),
        state: mr,
    });
}

/// The fixture's worktree, with the diff loaded for it.
fn ready(state: &mut crate::AppState, services: &Services, spawner: &SyncSpawner) -> WorktreeId {
    worktree(state, services, spawner);
    state
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|one| one.id.clone())
        .expect("a worktree")
}

fn act(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
    command: workspace::Command,
) {
    dispatch(Cmd::Workspace(command), state, services, spawner);
}

#[test]
fn a_branch_that_already_has_an_open_mr_is_offered_no_second_one() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let worktree = ready(&mut state, &services, &spawner);
    holding(&mut state, &worktree, MrState::Open);

    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::CreateMr,
    );
    assert!(state.pending.is_empty(), "nothing was sent");
    assert!(state.errors.is_empty(), "and nothing was reported");
}

#[test]
fn an_mr_that_is_no_longer_open_cannot_be_written_or_closed() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let worktree = ready(&mut state, &services, &spawner);
    holding(&mut state, &worktree, MrState::Merged);

    for command in [workspace::Command::UpdateMr, workspace::Command::CloseMr] {
        act(&mut state, &services, &spawner, command);
    }
    assert!(state.pending.is_empty(), "neither was sent");

    holding(&mut state, &worktree, MrState::Open);
    state
        .workspace
        .message
        .edit(&groove_types::Edit::Insert("fix: one".into()));
    act(
        &mut state,
        &services,
        &spawner,
        workspace::Command::UpdateMr,
    );
    assert!(!state.pending.is_empty(), "an open one can be written");
}
