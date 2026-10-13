//! Another worktree selected: what the last one showed goes with it, and nothing writes until it is read.

use groove_types::{Edit, WorktreeSpec};

use super::*;

/// A worktree with its commits read, and a second one on `other` beside it, the first selected.
fn two() -> (
    tempfile::TempDir,
    SyncSpawner,
    crate::AppState,
    Services,
    String,
) {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    worktree(&mut state, &services, &spawner);
    let (session, repo) = {
        let one = state.session.selected_worktree().expect("a worktree");
        (one.session.clone(), one.repo.clone())
    };
    let spec = WorktreeSpec {
        branch: Some("other".into()),
        ..WorktreeSpec::default()
    };
    let add = session::Command::AddWorktree {
        session,
        repo,
        spec,
    };
    send(&mut state, &services, &spawner, Cmd::Session(add));
    send(
        &mut state,
        &services,
        &spawner,
        Cmd::Workspace(workspace::Command::GetCommits),
    );
    let open = state.session.selected().expect("the session");
    let other = open.worktrees.iter().find(|one| one.branch == "other");
    let other = other.expect("the second worktree").path.clone();
    (home, spawner, state, services, other)
}

fn select(state: &mut crate::AppState, branch: &str) -> Cmd {
    let open = state.session.selected().expect("the session");
    let one = open
        .worktrees
        .iter()
        .find(|one| one.branch.ends_with(branch));
    Cmd::Session(session::Command::SelectWorktree {
        session: open.session.id.clone(),
        worktree: one.expect("the worktree").id.clone(),
    })
}

#[test]
fn a_commit_shown_goes_when_another_worktree_is_selected_and_stays_through_a_reload() {
    let (_home, spawner, mut state, services, other) = two();
    let sha = state.workspace.log[0].sha.clone();
    let open = Cmd::Workspace(workspace::Command::OpenCommit { sha });
    send(&mut state, &services, &spawner, open);
    let first = state
        .session
        .selected_worktree()
        .expect("a worktree")
        .path
        .clone();
    std::fs::write(std::path::Path::new(&first).join("new.txt"), "x\n").unwrap();
    send(
        &mut state,
        &services,
        &spawner,
        Cmd::Workspace(workspace::Command::Load),
    );
    assert!(state.workspace.readonly(), "a reload keeps the commit");
    assert!(
        !state
            .workspace
            .files
            .iter()
            .any(|one| one.path == "new.txt"),
        "and its files"
    );

    std::fs::write(std::path::Path::new(&other).join("b.txt"), "b\n").unwrap();
    let pick = select(&mut state, "other");
    send(&mut state, &services, &spawner, pick);
    assert!(
        !state.workspace.readonly(),
        "the other worktree is writable"
    );
    let paths: Vec<&str> = state
        .workspace
        .files
        .iter()
        .map(|one| one.path.as_str())
        .collect();
    assert_eq!(paths, ["b.txt"], "its own working tree");
}

#[test]
fn nothing_writes_before_the_worktree_selected_is_read() {
    let (_home, spawner, mut state, services, other) = two();
    let first = state
        .session
        .selected_worktree()
        .expect("a worktree")
        .path
        .clone();
    std::fs::write(std::path::Path::new(&first).join("a.txt"), "changed\n").unwrap();
    send(
        &mut state,
        &services,
        &spawner,
        Cmd::Workspace(workspace::Command::Load),
    );
    let kept = std::path::Path::new(&other).join("a.txt");
    std::fs::write(&kept, "mine\n").unwrap();
    let pick = select(&mut state, "other");
    dispatch(pick, &mut state, &services, &spawner);
    for one in [
        workspace::Command::DiscardAll,
        workspace::Command::Edit(Edit::Insert("x".into())),
        workspace::Command::SaveFile,
    ] {
        dispatch(Cmd::Workspace(one), &mut state, &services, &spawner);
    }
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert_eq!(std::fs::read_to_string(&kept).unwrap(), "mine\n");
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}
