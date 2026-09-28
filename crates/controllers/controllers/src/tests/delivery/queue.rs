//! The review queue as the controller reads it: from the clones on disk, into the column.

use super::*;

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
        review: None,
    }
}

#[test]
fn an_empty_pool_asks_nothing_and_leaves_the_column_empty() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    state.delivery.reviews = vec![mr("acme/groove")];
    dispatch(
        Cmd::Delivery(delivery::Command::ReviewQueue),
        &mut state,
        &services,
        &spawner,
    );
    spawner.drain(&mut state, &services);
    assert!(
        state.delivery.reviews.is_empty(),
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
        Cmd::Delivery(delivery::Command::ReviewQueue),
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
