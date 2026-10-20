//! The worktree reads and the watch a worktree keeps.

use super::*;

#[test]
fn closing_the_last_worktree_stops_watching_it() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let id = explorer(&mut state, &services, &spawner);
    let only = first_worktree(&mut state, &services, &spawner, &id);

    crate::workspace::follow(&mut state, &spawner);
    spawner.drain(&mut state, &services);
    assert_eq!(state.workspace.watching.as_ref(), Some(&only.id));

    close(&mut state, &services, &spawner, &id, &only.id, true);
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert!(state.workspace.watching.is_none(), "nothing watches it");
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    assert!(!std::path::Path::new(&only.path).exists());
}

#[test]
fn reads_asked_while_one_is_out_wait_for_it_and_make_one_more() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let id = explorer(&mut state, &services, &spawner);
    first_worktree(&mut state, &services, &spawner, &id);
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());

    for _ in 0..4 {
        crate::workspace::load(&mut state, &spawner);
    }
    let reading = |s: &crate::AppState| {
        s.pending
            .iter()
            .filter(|job| job.label == "changed files")
            .count()
    };
    assert_eq!(reading(&state), 1, "one read out, whatever asked");
    assert!(state.workspace.again, "the rest owe it one more");
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert!(state.workspace.loading.is_none() && !state.workspace.again);
}

#[test]
fn a_worktree_that_is_gone_is_read_without_an_error() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let id = explorer(&mut state, &services, &spawner);
    let only = first_worktree(&mut state, &services, &spawner, &id);

    std::fs::remove_dir_all(&only.path).unwrap();
    crate::workspace::load(&mut state, &spawner);
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert!(state.errors.is_empty(), "{:?}", state.errors);
}

#[test]
fn a_repo_added_by_its_url_takes_the_clone_the_pool_holds() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let id = explorer(&mut state, &services, &spawner);
    let add = Command::AddRepo {
        session: id.clone(),
        name: "git@gitlab.example.com:g/mayo.git".into(),
        spec: WorktreeSpec::default(),
    };
    dispatch(session_cmd(add), &mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| {
        s.session.get(&id).is_some_and(|o| !o.worktrees.is_empty()) || !s.errors.is_empty()
    });
    assert!(
        state.errors.is_empty(),
        "no second clone: {:?}",
        state.errors
    );
    let open = state.session.get(&id).unwrap();
    assert_eq!(open.repos[0].id, RepoId::new(REPO));
}
