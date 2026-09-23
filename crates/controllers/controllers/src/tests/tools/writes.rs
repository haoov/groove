//! What a write does: it waits, then it runs or it does not.

use groove_agent_service::{Call, Reply};
use serde_json::json;

use super::{asked, changed, settled, stage, waited, waiting};
use crate::SyncSpawner;
use crate::tests::fixture::{self, services, sh, state};

#[test]
fn a_tool_groove_does_not_answer_yet_says_so() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        "gh-nothing",
        "get_mr_threads",
        json!({}),
    );
    assert!(answer.failed);
    assert!(answer.text.contains("get_mr_threads"), "{}", answer.text);
}

#[test]
fn a_write_waits_for_the_user_and_says_so_on_the_row() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, worktree, dir) = changed(&mut state, &services, &spawner);
    stage(&dir);

    let answer = waited(
        &mut state,
        &services,
        &spawner,
        &id,
        "git_commit",
        json!({ "worktree_id": worktree, "message": "fix: one" }),
    );
    assert!(answer.is_none(), "it runs only once the user says so");
    let asks = waiting(&state, &id);
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].op, "git_commit");
    assert_eq!(asks[0].subject, "fix: one");
}

#[test]
fn the_write_the_user_allows_runs_and_answers_its_agent() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, worktree, dir) = changed(&mut state, &services, &spawner);
    stage(&dir);

    let (reply, mut answered) = Reply::new();
    crate::tools::answer(
        &mut state,
        &services,
        &spawner,
        Call {
            session: id.clone(),
            tool: "git_commit".into(),
            arguments: json!({ "worktree_id": worktree, "message": "fix: one" }),
            reply,
        },
    );
    let ask = waiting(&state, &id)[0].id.clone();
    crate::tools::allow(&mut state, &services, &spawner, &ask);
    let answer = settled(&spawner, &services, &mut state, &mut answered);
    assert!(!answer.failed, "{}", answer.text);
    assert!(answer.text.contains("fix: one"), "{}", answer.text);
    assert!(waiting(&state, &id).is_empty(), "the row is clear");
    assert!(sh(std::path::Path::new(&dir), &["log", "-1", "--pretty=%s"]).contains("fix: one"));
}

#[test]
fn the_write_the_user_refuses_never_runs() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, worktree, dir) = changed(&mut state, &services, &spawner);
    stage(&dir);

    let (reply, mut answered) = Reply::new();
    crate::tools::answer(
        &mut state,
        &services,
        &spawner,
        Call {
            session: id.clone(),
            tool: "git_commit".into(),
            arguments: json!({ "worktree_id": worktree, "message": "fix: one" }),
            reply,
        },
    );
    let ask = waiting(&state, &id)[0].id.clone();
    crate::tools::refuse(&mut state, &ask);
    let answer = answered.try_recv().expect("an answer");
    assert!(answer.failed);
    assert!(answer.text.contains("refused"), "{}", answer.text);
    assert!(waiting(&state, &id).is_empty());
    assert!(!sh(std::path::Path::new(&dir), &["log", "-1", "--pretty=%s"]).contains("fix: one"));
}

#[test]
fn a_session_that_auto_approves_never_asks() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, worktree, dir) = changed(&mut state, &services, &spawner);
    stage(&dir);
    state
        .agent
        .auto_approve(&groove_types::SessionId::new(&id), true);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "git_commit",
        json!({ "worktree_id": worktree, "message": "fix: two" }),
    );
    assert!(!answer.failed, "{}", answer.text);
    assert!(waiting(&state, &id).is_empty());
    assert!(sh(std::path::Path::new(&dir), &["log", "-1", "--pretty=%s"]).contains("fix: two"));
}

#[test]
fn a_commit_with_nothing_staged_is_an_error_not_a_commit() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);
    state
        .agent
        .auto_approve(&groove_types::SessionId::new(&id), true);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "git_commit",
        json!({ "worktree_id": worktree, "message": "fix: three" }),
    );
    assert!(answer.failed);
    assert!(answer.text.contains("staged"), "{}", answer.text);
}

#[test]
fn a_session_that_ends_leaves_no_agent_waiting() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);

    let (reply, mut answered) = Reply::new();
    crate::tools::answer(
        &mut state,
        &services,
        &spawner,
        Call {
            session: id.clone(),
            tool: "git_push".into(),
            arguments: json!({ "worktree_id": worktree }),
            reply,
        },
    );
    crate::agent::end(&mut state, &groove_types::SessionId::new(&id));
    let answer = answered.try_recv().expect("an answer");
    assert!(answer.failed);
    assert!(answer.text.contains("closed"), "{}", answer.text);
}
