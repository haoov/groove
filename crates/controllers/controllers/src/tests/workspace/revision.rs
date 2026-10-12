//! A repo a git URL names, shown from an Application: added when missing, and a commit of it shown.

use super::*;

const URL: &str = "https://gitlab.example.com/g/mayo.git";

fn send(
    state: &mut crate::AppState,
    services: &Services,
    spawner: &SyncSpawner,
    command: workspace::Command,
) {
    dispatch(Cmd::Workspace(command), state, services, spawner);
    until(spawner, services, state, |s| {
        s.pending.is_empty() && s.workspace.revision.is_none() || !s.errors.is_empty()
    });
}

#[test]
fn a_commit_pushed_after_the_worktree_was_cut_is_fetched_and_shown() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    worktree(&mut state, &services, &spawner);
    let seed = home.path().join("seed");
    std::fs::write(seed.join("c.txt"), "c\n").unwrap();
    sh(&seed, &["add", "."]);
    sh(&seed, &["commit", "-m", "deployed"]);
    sh(&seed, &["push", "origin", "main"]);
    let sha = sh(&seed, &["rev-parse", "HEAD"]).trim().to_string();

    let show = workspace::Command::ShowRepo {
        repo: URL.into(),
        target: Some("main".into()),
        sha: Some(sha.clone()),
    };
    send(&mut state, &services, &spawner, show);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    let shown = state.workspace.commit.as_ref();
    assert_eq!(shown.map(|one| one.message.as_str()), Some("deployed"));
    let paths: Vec<&str> = state
        .workspace
        .files
        .iter()
        .map(|one| one.path.as_str())
        .collect();
    assert_eq!(paths, ["c.txt"]);
}

#[test]
fn a_repo_the_session_lacks_is_added_from_the_pool_and_selected() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let open = session::Command::OpenExplorer {
        title: Some("look".into()),
    };
    dispatch(Cmd::Session(open), &mut state, &services, &spawner);
    let id = state.session.selected.clone().expect("a session");
    until(&spawner, &services, &mut state, |s| {
        s.agent.agent(&id).is_some()
    });

    let show = workspace::Command::ShowRepo {
        repo: URL.into(),
        target: Some("release/1.0".into()),
        sha: None,
    };
    send(&mut state, &services, &spawner, show);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    let selected = state.session.selected_worktree().expect("a worktree");
    assert_eq!(selected.repo.as_str(), "gitlab.example.com/g/mayo");
    assert_eq!(
        selected.base_ref.as_deref(),
        Some("release/1.0"),
        "cut from the branch the source follows"
    );
}

#[test]
fn a_commit_that_cannot_be_read_ends_the_revision_and_says_why() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    worktree(&mut state, &services, &spawner);
    let show = workspace::Command::ShowRepo {
        repo: URL.into(),
        target: Some("main".into()),
        sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
    };
    send(&mut state, &services, &spawner, show);
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert!(!state.errors.is_empty(), "the read failed");
    assert_eq!(state.workspace.revision, None, "and the revision ended");
}

#[test]
fn a_revision_asked_in_one_session_is_dropped_once_another_is_selected() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    let before = worktree(&mut state, &services, &spawner);
    state.workspace.revision = Some(groove_workspace_service::Revision {
        session: groove_types::SessionId::new("elsewhere"),
        slug: "gitlab.example.com/g/mayo".into(),
        sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
        reading: false,
    });
    send(&mut state, &services, &spawner, workspace::Command::Load);
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert_eq!(state.workspace.revision, None);
    assert!(
        state.errors.is_empty(),
        "nothing read here: {:?}",
        state.errors
    );
    let selected = state
        .session
        .selected_worktree()
        .map(|one| one.path.clone());
    assert_eq!(selected.as_deref(), Some(before.as_str()));
}

/// A revision of a repo the session lacks, asked while another one's work is out.
fn newer(state: &crate::AppState) -> groove_workspace_service::Revision {
    groove_workspace_service::Revision {
        session: state.session.selected.clone().expect("a session"),
        slug: "gitlab.example.com/g/other".into(),
        sha: Some("89abcdef0123456789abcdef0123456789abcdef".into()),
        reading: false,
    }
}

#[test]
fn a_read_that_lands_after_a_newer_revision_was_asked_leaves_the_newer_one() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    worktree(&mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    let show = workspace::Command::ShowRepo {
        repo: URL.into(),
        target: Some("main".into()),
        sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
    };
    dispatch(Cmd::Workspace(show), &mut state, &services, &spawner);
    let asked = newer(&state);
    state.workspace.revision = Some(asked.clone());
    spawner.drain(&mut state, &services);
    assert_eq!(state.workspace.revision, Some(asked));
}

#[test]
fn an_add_that_fails_after_a_newer_revision_was_asked_leaves_the_newer_one() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    pooled_clone(home.path());
    worktree(&mut state, &services, &spawner);
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    let show = workspace::Command::ShowRepo {
        repo: "https://gitlab.example.com/g/nowhere.git".into(),
        target: Some("main".into()),
        sha: None,
    };
    dispatch(Cmd::Workspace(show), &mut state, &services, &spawner);
    let asked = newer(&state);
    state.workspace.revision = Some(asked.clone());
    spawner.drain(&mut state, &services);
    assert!(!state.errors.is_empty(), "the add failed");
    assert_eq!(state.workspace.revision, Some(asked));
}
