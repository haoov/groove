//! The review queue: which hosts are asked, and what the pool adds to the answer.

use groove_types::PoolEntry;

use super::*;
use crate::workspace::queue::{hosts, local};

fn pooled(slug: &str, path: &str) -> PoolEntry {
    PoolEntry {
        slug: slug.into(),
        path: std::path::PathBuf::from(path),
    }
}

fn mr(project: &str) -> groove_types::ReviewMr {
    groove_types::ReviewMr {
        forge: groove_types::Forge::Github,
        project: project.into(),
        iid: 7,
        title: "fix: one".into(),
        author: "someone".into(),
        source_branch: "fix/one".into(),
        target_branch: "main".into(),
        draft: false,
        web_url: "https://github.com/acme/groove/pull/7".into(),
        updated_at: groove_types::Timestamp::new(0),
        local_path: None,
        approved: false,
    }
}

#[test]
fn every_host_the_pool_knows_is_asked_once_whichever_forge_it_carries() {
    let pool = vec![
        pooled("github.com/acme/groove", "/pool/groove"),
        pooled("github.com/acme/other", "/pool/other"),
        pooled("gitlab.wiremind.io/devops/charts", "/pool/charts"),
    ];
    assert_eq!(
        hosts(&pool),
        ["github.com", "gitlab.wiremind.io"],
        "each host once, both forges"
    );
}

#[test]
fn an_mr_whose_repo_is_in_the_pool_carries_where_it_sits() {
    let pool = vec![
        pooled("github.com/acme/groove", "/pool/groove"),
        pooled("github.com/other/thing", "/pool/thing"),
    ];
    let found = local(mr("acme/groove"), &pool);
    assert_eq!(found.local_path.as_deref(), Some("/pool/groove"));

    let elsewhere = local(mr("nobody/else"), &pool);
    assert_eq!(
        elsewhere.local_path, None,
        "one the pool does not hold is cloned when it is opened"
    );
}

#[test]
fn an_empty_pool_asks_nothing_and_leaves_the_column_empty() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    state.workspace.reviews = vec![mr("acme/groove")];
    dispatch(
        Cmd::Workspace(workspace::Command::ReviewQueue),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(hosts(&[]).is_empty());
    assert!(
        state.workspace.reviews.is_empty(),
        "what was there is not kept"
    );
}

#[test]
fn the_queue_asks_the_clones_on_disk_and_not_what_the_ui_listed() {
    let home = tempfile::tempdir().unwrap();
    let clone = pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    assert!(state.session.pool.is_empty(), "nothing has listed it yet");

    dispatch(
        Cmd::Workspace(workspace::Command::ReviewQueue),
        &mut state,
        &services,
        &spawner,
    );
    until(&spawner, &services, &mut state, |s| s.pending.is_empty());
    assert!(clone.exists());
    let said = format!("{:?}", state.errors);
    assert!(
        said.contains("gitlab.example.com"),
        "the host of the clone was asked: {said}"
    );
}
