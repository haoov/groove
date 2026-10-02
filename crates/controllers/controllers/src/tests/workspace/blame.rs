//! Who last changed each line of the open file, and the commit a blame opens.

use super::*;

fn blame(state: &mut crate::AppState, services: &Services, spawner: &SyncSpawner) {
    let path = "a.txt".to_string();
    dispatch(
        Cmd::Workspace(workspace::Command::Blame { path }),
        state,
        services,
        spawner,
    );
    until(spawner, services, state, |s| s.pending.is_empty());
}

/// Whether each line of `a.txt` is still to be committed, as its kept blame says.
fn uncommitted(state: &crate::AppState) -> Vec<bool> {
    let worktree = state
        .session
        .selected_worktree()
        .expect("a worktree")
        .id
        .clone();
    let read = state.workspace.read_of("a.txt");
    let lines = state
        .workspace
        .blames
        .of(&worktree, "a.txt", read)
        .expect("a blame kept");
    lines.iter().map(|one| one.uncommitted).collect()
}

#[test]
fn the_blame_reads_the_buffer_with_its_edits() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    pooled_clone(home.path());
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    editing(&mut state, &services, &spawner);
    blame(&mut state, &services, &spawner);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert_eq!(
        uncommitted(&state),
        [false, true],
        "the seed's line, then the one on disk"
    );

    edit(&mut state, &services, &spawner, &[Edit::Insert("x".into())]);
    blame(&mut state, &services, &spawner);
    assert_eq!(uncommitted(&state), [true, true], "the line just typed on");
}

#[test]
fn a_commit_only_a_blame_names_opens_as_its_change() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    pooled_clone(home.path());
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    editing(&mut state, &services, &spawner);
    blame(&mut state, &services, &spawner);
    let worktree = state
        .session
        .selected_worktree()
        .expect("a worktree")
        .id
        .clone();
    let read = state.workspace.read_of("a.txt");
    let line = &state
        .workspace
        .blames
        .of(&worktree, "a.txt", read)
        .expect("a blame")[0];
    let sha = line.sha.clone();
    assert!(state.workspace.log.is_empty(), "the history is not read");

    let open = workspace::Command::OpenCommit { sha: sha.clone() };
    dispatch(Cmd::Workspace(open), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    let shown = state
        .workspace
        .commit
        .as_ref()
        .expect("the commit is shown");
    assert_eq!(
        (shown.sha.as_str(), shown.message.as_str()),
        (sha.as_str(), "first")
    );
}
