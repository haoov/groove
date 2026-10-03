//! What the reads answer.

use serde_json::{Value, json};

use super::{asked, changed, said};
use crate::tests::fixture::{self, worktree};

#[test]
fn the_active_task_is_the_session_the_call_came_from() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        "gh-nothing",
        "get_active_task",
        json!({}),
    );
    assert!(answer.failed, "{}", answer.text);
    assert!(
        answer.text.contains(crate::tools::NO_SESSION),
        "{}",
        answer.text
    );
}

#[test]
fn the_pool_says_which_repos_the_session_already_has() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
fn the_diff_says_what_changed_in_every_worktree_of_the_task() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let commits = said["worktrees"][0]["commits"]
        .as_array()
        .expect("its commits");
    let subjects: Vec<&str> = commits
        .iter()
        .filter_map(|one| one["message"].as_str())
        .collect();
    assert!(
        subjects.iter().any(|one| one.starts_with("first")),
        "{said}"
    );
}

#[test]
fn a_file_is_read_as_the_worktree_holds_it() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    assert!(
        answer.text.contains(crate::tools::NO_WORKTREE),
        "{}",
        answer.text
    );
}

#[test]
fn a_worktree_with_no_merge_request_says_none() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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

#[test]
fn the_skills_a_session_is_offered_are_the_ones_for_its_kind() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let id = state.session.selected.clone();
    assert!(id.is_none(), "nothing is open yet");
    fixture::pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    assert!(!dir.is_empty());
    let id = state.session.selected.clone().expect("a session");

    crate::agent::skills::list(&mut state, &spawner);
    spawner.drain(&mut state, &services);
    let answer = asked(
        &mut state,
        &services,
        &spawner,
        id.as_str(),
        "list_skills",
        json!({}),
    );
    let said = said(&answer);
    let ids: Vec<&str> = said["skills"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|one| one["id"].as_str())
        .collect();
    assert!(ids.contains(&"groove:create-task"), "{ids:?}");
    assert!(
        !ids.contains(&"groove:save-task"),
        "an explorer has no task to land: {ids:?}"
    );
}

#[test]
fn the_agent_writes_a_skill_of_the_users_own_and_reads_it_back() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let dir = worktree(&mut state, &services, &spawner);
    assert!(!dir.is_empty());
    let id = state.session.selected.clone().expect("a session");
    crate::tests::fixture::auto_approve(&mut state, &id);
    crate::agent::skills::list(&mut state, &spawner);
    spawner.drain(&mut state, &services);

    let body = "---\ndescription: Ship it.\ngroove-label: ship it\n---\n\nDo it.\n";
    let wrote = asked(
        &mut state,
        &services,
        &spawner,
        id.as_str(),
        "save_user_skill",
        json!({ "name": "ship-it", "body": body }),
    );
    assert!(!wrote.failed, "{}", wrote.text);
    assert!(wrote.text.contains("user:ship-it"), "{}", wrote.text);

    let read = asked(
        &mut state,
        &services,
        &spawner,
        id.as_str(),
        "read_user_skill",
        json!({ "name": "ship-it" }),
    );
    assert_eq!(read.text, body);
    assert!(
        state
            .agent
            .skills
            .iter()
            .any(|one| one.id == "user:ship-it"),
        "the list holds it at once"
    );
}

#[test]
fn the_task_body_is_read_again_from_its_source() {
    let (_runtime, server) = crate::tests::tasks::answering();
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let host = format!("http://{}", server.address());
    state.config.config.as_mut().expect("a config").github =
        Some(serde_json::from_value(crate::tests::tasks::source(&host)).expect("the source"));
    crate::dispatch(
        crate::Command::Task(crate::task::Command::Load),
        &mut state,
        &services,
        &spawner,
    );
    crate::tests::fixture::until(&spawner, &services, &mut state, |s| {
        !s.task.tasks.is_empty()
    });

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        "gh-haoov-groove-50",
        "get_task_body",
        json!({ "task_id": "gh-haoov-groove-50" }),
    );
    let said = said(&answer);
    assert_eq!(said["title"], "Harden Groove");
    assert_eq!(said["body_markdown"], "Close the gates.");
    assert_eq!(
        state.task.body("gh-haoov-groove-50"),
        Some("Close the gates."),
        "and what was read is kept"
    );
}

#[test]
fn a_session_with_no_task_of_its_own_is_told_so() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    worktree(&mut state, &services, &spawner);
    let id = state.session.selected.clone().unwrap();

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        id.as_str(),
        "get_task_body",
        json!({}),
    );
    assert!(answer.failed, "{}", answer.text);
    assert!(answer.text.contains("works no task"), "{}", answer.text);
}

#[test]
fn a_source_with_no_template_answers_an_empty_one() {
    let (_runtime, server) = crate::tests::tasks::answering();
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let host = format!("http://{}", server.address());
    state.config.config.as_mut().expect("a config").github =
        Some(serde_json::from_value(crate::tests::tasks::source(&host)).expect("the source"));

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        "gh-haoov-groove-50",
        "get_task_template",
        json!({}),
    );
    assert_eq!(said(&answer)["template_markdown"], "");
    assert_eq!(
        said(&answer)["file_at"]["provider"],
        "github",
        "and where a new one is filed"
    );
}

#[test]
fn a_template_asked_of_a_source_that_is_not_set_up_is_refused() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        "gh-nothing",
        "get_task_template",
        json!({ "provider": "notion" }),
    );
    assert!(answer.failed, "{}", answer.text);
}

#[test]
fn the_notes_left_on_the_session_are_read_back_with_their_ids() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);
    crate::tests::fixture::auto_approve(&mut state, &groove_types::SessionId::new(&id));
    let note =
        json!({ "worktree_id": worktree, "path": "a.txt", "line": 1, "content": "issue: one" });
    asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "create_annotation",
        note,
    );
    fixture::until(&spawner, &services, &mut state, |s| {
        !s.delivery
            .notes_of(&groove_types::SessionId::new(&id))
            .unwrap_or_default()
            .is_empty()
    });

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "get_annotations",
        json!({}),
    );
    let notes = said(&answer)["annotations"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert_eq!(
        notes[0]["id"],
        state
            .delivery
            .notes_of(&groove_types::SessionId::new(&id))
            .unwrap_or_default()[0]
            .id
            .as_str()
    );
    assert_eq!(notes[0]["content"], "issue: one");
}

#[test]
fn the_forge_reads_name_a_worktree_of_an_open_session() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    worktree(&mut state, &services, &spawner);
    let id = state.session.selected.clone().unwrap();
    for tool in ["get_mr_threads", "get_mr_ci"] {
        let args = json!({ "worktree_id": "wt-nowhere" });
        let answer = asked(&mut state, &services, &spawner, id.as_str(), tool, args);
        assert!(answer.failed, "{tool}: {}", answer.text);
    }
}
