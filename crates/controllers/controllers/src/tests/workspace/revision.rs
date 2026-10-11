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
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    pooled_clone(home.path());
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
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
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    pooled_clone(home.path());
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
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
