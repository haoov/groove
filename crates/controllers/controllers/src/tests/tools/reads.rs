//! What the reads answer.

use serde_json::{Value, json};

use super::{asked, changed, said};
use crate::SyncSpawner;
use crate::tests::fixture::{self, services, state, worktree};

#[test]
fn the_active_task_is_the_session_the_call_came_from() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    worktree(&mut state, &services, &spawner);
    let id = state.session.selected.clone().unwrap();

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        id.as_str(),
        "get_active_task",
        json!({}),
    );
    let said = said(&answer);
    assert_eq!(said["session"]["id"], id.as_str());
    assert_eq!(said["session"]["title"], "try mayo");
    assert_eq!(said["repos"].as_array().unwrap().len(), 1);
    let worktrees = said["worktrees"].as_array().unwrap();
    assert_eq!(worktrees.len(), 1);
    assert!(
        worktrees[0]["branch"]
            .as_str()
            .unwrap()
            .starts_with("explorer/")
    );
    assert!(worktrees[0]["path"].is_string());
}

#[test]
fn a_call_from_no_open_session_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        "gh-nothing",
        "get_active_task",
        json!({}),
    );
    assert!(answer.failed, "{}", answer.text);
}

#[test]
fn the_pool_says_which_repos_the_session_already_has() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    worktree(&mut state, &services, &spawner);
    let id = state.session.selected.clone().unwrap();

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        id.as_str(),
        "list_repos",
        json!({}),
    );
    let said = said(&answer);
    assert_eq!(said["count"], 1);
    assert_eq!(said["repos"][0]["attached"], true);
    assert!(said["repos"][0]["local_path"].is_string());
}

#[test]
fn no_open_file_is_said_as_none() {
    let home = tempfile::tempdir().unwrap();
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        "gh-nothing",
        "get_open_file",
        json!({}),
    );
    assert_eq!(said(&answer)["open_file"], Value::Null);
}

#[test]
fn the_diff_says_what_changed_in_every_worktree_of_the_task() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "get_task_diff",
        json!({}),
    );
    let said = said(&answer);
    let worktrees = said["worktrees"].as_array().unwrap();
    assert_eq!(worktrees.len(), 1);
    assert_eq!(worktrees[0]["worktree_id"], worktree);
    assert_eq!(worktrees[0]["files"][0]["path"], "a.txt");
}

#[test]
fn the_status_counts_what_the_worktree_holds() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "get_status",
        json!({}),
    );
    let said = said(&answer);
    let told = &said["worktrees"][0];
    assert_eq!(told["worktree_id"], worktree);
    assert_eq!(told["modified"], 1);
    assert_eq!(told["staged"], 0);
}

#[test]
fn the_log_holds_the_commits_the_branch_stands_on() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, _, _) = changed(&mut state, &services, &spawner);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "get_commit_log",
        json!({ "limit": 5 }),
    );
    let said = said(&answer);
    assert!(said["worktrees"][0]["commits"].is_array(), "{said}");
}

#[test]
fn a_file_is_read_as_the_worktree_holds_it() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "read_file",
        json!({ "worktree_id": worktree, "path": "a.txt" }),
    );
    assert_eq!(said(&answer)["text"], "one\ntwo\n");
}

#[test]
fn a_worktree_no_session_holds_is_refused() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, _, _) = changed(&mut state, &services, &spawner);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "read_file",
        json!({ "worktree_id": "w-nothing", "path": "a.txt" }),
    );
    assert!(answer.failed, "{}", answer.text);
}

#[test]
fn a_worktree_with_no_merge_request_says_none() {
    let home = tempfile::tempdir().unwrap();
    fixture::pooled_clone(home.path());
    let spawner = SyncSpawner::new().unwrap();
    let services = services(&spawner, home.path());
    let mut state = state(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "get_mr_state",
        json!({ "worktree_id": worktree }),
    );
    assert_eq!(said(&answer)["mr"], Value::Null);
}
