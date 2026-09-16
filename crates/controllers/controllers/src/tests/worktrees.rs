use groove_types::{RepoId, WorktreeSpec};

use crate::session::Command;
use crate::tests::fixture::{self, pooled_clone, services, sh, state, until};
use crate::{AppState, Command as Cmd, Services, SyncSpawner, dispatch};

const REPO: &str = "gitlab.example.com/g/mayo";

/// An explorer open with the fixture's clone in the pool; returns its id.
fn explorer(
    state: &mut AppState,
    services: &Services,
    spawner: &SyncSpawner,
) -> groove_types::SessionId {
    dispatch(
        Cmd::Session(Command::OpenExplorer {
            title: Some("try mayo".into()),
        }),
        state,
        services,
        spawner,
    );
    let id = state.session.selected.clone().unwrap();
    until(spawner, services, state, |s| s.agent.agent(&id).is_some());
    id
}

fn session_cmd(command: Command) -> Cmd {
    Cmd::Session(command)
}

#[test]
fn add_repo_cuts_the_first_worktree_and_lists_the_pool() {
    let home = tempfile::tempdir().unwrap();
    let clone = pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = explorer(&mut state, &services, &spawner);

    dispatch(
        session_cmd(Command::ListRepos),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert_eq!(state.session.pool.len(), 1);
    assert_eq!(state.session.pool[0].slug, REPO);
    assert_eq!(state.session.pool[0].path, clone);

    dispatch(
        session_cmd(Command::AddRepo {
            session: id.clone(),
            name: "mayo".into(),
            spec: WorktreeSpec::default(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.session.get(&id).is_some_and(|o| !o.worktrees.is_empty())
    });
    let open = state.session.get(&id).unwrap();
    assert_eq!(open.repos.len(), 1);
    assert_eq!(open.repos[0].id, RepoId::new(REPO));
    let wt = &open.worktrees[0];
    assert_eq!(wt.branch, "explorer/try-mayo");
    assert!(std::path::Path::new(&wt.path).join("a.txt").is_file());
    assert_eq!(open.state.selected_worktree.as_ref(), Some(&wt.id));
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(state.notes.is_empty(), "{:?}", state.notes);

    dispatch(
        session_cmd(Command::ListBranches {
            repo: RepoId::new(REPO),
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert_eq!(state.session.branches[0].1, ["main", "release/1.0"]);
}

#[test]
fn a_second_worktree_a_selection_and_a_close_survive_a_restart() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = explorer(&mut state, &services, &spawner);
    let repo = RepoId::new(REPO);

    dispatch(
        session_cmd(Command::AddRepo {
            session: id.clone(),
            name: "mayo".into(),
            spec: WorktreeSpec::default(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    let spec = WorktreeSpec {
        branch: Some("fix/x".into()),
        target: Some("release/1.0".into()),
        track_remote: None,
    };
    dispatch(
        session_cmd(Command::AddWorktree {
            session: id.clone(),
            repo: repo.clone(),
            spec,
        }),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| {
        s.session.get(&id).is_some_and(|o| o.worktrees.len() == 2)
    });
    let second = state
        .session
        .get(&id)
        .unwrap()
        .worktrees
        .iter()
        .find(|w| w.branch == "fix/x")
        .unwrap()
        .clone();
    assert_eq!(second.base_ref.as_deref(), Some("release/1.0"));

    dispatch(
        session_cmd(Command::SelectWorktree {
            session: id.clone(),
            worktree: second.id.clone(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert_eq!(
        state
            .session
            .get(&id)
            .unwrap()
            .state
            .selected_worktree
            .as_ref(),
        Some(&second.id)
    );

    let mut fresh = fixture::state(home.path());
    dispatch(
        session_cmd(Command::Restore),
        &mut fresh,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut fresh, |s| {
        s.session.get(&id).is_some_and(|o| o.worktrees.len() == 2)
    });
    let open = fresh.session.get(&id).unwrap();
    assert_eq!(open.repos.len(), 1);
    assert_eq!(
        open.state.selected_worktree.as_ref(),
        Some(&second.id),
        "the selection was persisted"
    );

    let first = open
        .worktrees
        .iter()
        .find(|w| w.branch == "explorer/try-mayo")
        .unwrap()
        .clone();
    std::fs::write(std::path::Path::new(&second.path).join("a.txt"), "dirty\n").unwrap();
    dispatch(
        session_cmd(Command::CloseWorktree {
            session: id.clone(),
            worktree: second.id.clone(),
            force: false,
        }),
        &mut fresh,
        &services,
        &spawner,
    );
    spawner.drain(&mut fresh, &services);
    assert_eq!(fresh.errors.len(), 1, "a dirty worktree is refused");
    assert_eq!(fresh.errors[0].kind, groove_types::ErrorKind::Conflict);
    fresh.errors.clear();

    dispatch(
        session_cmd(Command::CloseWorktree {
            session: id.clone(),
            worktree: second.id.clone(),
            force: true,
        }),
        &mut fresh,
        &services,
        &spawner,
    );
    spawner.drain(&mut fresh, &services);
    let open = fresh.session.get(&id).unwrap();
    assert_eq!(open.worktrees.len(), 1);
    assert_eq!(
        open.repos.len(),
        1,
        "the repo stays while a worktree remains"
    );
    assert_eq!(
        open.state.selected_worktree.as_ref(),
        Some(&first.id),
        "the selection moved"
    );
    assert!(!std::path::Path::new(&second.path).exists());

    dispatch(
        session_cmd(Command::RemoveRepo {
            session: id.clone(),
            repo: repo.clone(),
            force: false,
        }),
        &mut fresh,
        &services,
        &spawner,
    );
    spawner.drain(&mut fresh, &services);
    let open = fresh.session.get(&id).unwrap();
    assert!(open.worktrees.is_empty());
    assert!(open.repos.is_empty());
    assert!(open.state.selected_worktree.is_none());
    assert!(fresh.errors.is_empty(), "{:?}", fresh.errors);
    let clone = home.path().join("code/main").join(REPO);
    assert!(
        !sh(&clone, &["branch", "--list", "fix/x"]).contains("fix/x"),
        "the local branch goes with the worktree"
    );
    until(&spawner, &services, &mut fresh, |s| s.pending.is_empty());
}

#[test]
fn an_unknown_repo_or_target_is_an_error_not_a_worktree() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = explorer(&mut state, &services, &spawner);

    dispatch(
        session_cmd(Command::AddRepo {
            session: id.clone(),
            name: "nope".into(),
            spec: WorktreeSpec::default(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert_eq!(state.errors.len(), 1);
    assert_eq!(state.errors[0].kind, groove_types::ErrorKind::NotFound);

    let spec = WorktreeSpec {
        branch: None,
        target: Some("ghost".into()),
        track_remote: None,
    };
    dispatch(
        session_cmd(Command::AddRepo {
            session: id.clone(),
            name: "mayo".into(),
            spec,
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert_eq!(state.errors.len(), 2);
    assert!(
        state.errors[1].message.contains("release/1.0"),
        "{}",
        state.errors[1].message
    );
    assert!(state.session.get(&id).unwrap().worktrees.is_empty());
}

#[test]
fn a_git_url_is_cloned_into_the_pool_and_a_bad_name_is_not() {
    let home = tempfile::tempdir().unwrap();
    pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let id = explorer(&mut state, &services, &spawner);

    let url = "ssh://git@gitlab.example.com/other/proj.git";
    dispatch(
        session_cmd(Command::AddRepo {
            session: id.clone(),
            name: url.into(),
            spec: WorktreeSpec::default(),
        }),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert_eq!(
        state.errors.len(),
        1,
        "no network: the clone itself fails, not the resolution"
    );
    assert_eq!(
        state.errors[0].kind,
        groove_types::ErrorKind::Git,
        "{}",
        state.errors[0].message
    );
    assert!(state.session.get(&id).unwrap().repos.is_empty());
}
