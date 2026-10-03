//! What a write does: it waits, then it runs or it does not.

use groove_agent_service::{Call, Reply};
use serde_json::json;

use super::{asked, changed, settled, stage, waited, waiting};
use crate::tests::fixture::{self, sh, until};

#[test]
fn a_tool_groove_does_not_answer_yet_says_so() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        "gh-nothing",
        "get_nothing",
        json!({}),
    );
    assert!(answer.failed);
    assert!(answer.text.contains("get_nothing"), "{}", answer.text);
}

#[test]
fn a_write_waits_for_the_user_and_says_so_on_the_row() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
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
    let answer = settled(&spawner, &services, &mut state, &mut answered);
    assert!(answer.failed);
    assert!(answer.text.contains("closed"), "{}", answer.text);
}

#[test]
fn every_tool_groove_lists_is_one_it_answers_and_a_write_with_nothing_to_act_on_refuses() {
    let (_home, spawner, services, mut state) = crate::tests::fixture::fresh();
    let nothing = groove_types::SessionId::new("gh-nothing");
    let launch = state.agent.launch();
    let none = Err(groove_types::Error::invalid("no terminal in this test"));
    state.agent.started(
        (nothing.clone(), launch),
        none,
        groove_types::Timestamp::now(),
    );
    state.agent.auto_approve(&nothing, true);

    let (mut unanswered, mut took_nothing) = (Vec::new(), Vec::new());
    for tool in groove_agent_service::tools::all() {
        let answer = waited(
            &mut state,
            &services,
            &spawner,
            "gh-nothing",
            tool.name,
            json!({}),
        );
        let answer = answer.unwrap_or_else(|| panic!("{} answered", tool.name));
        if answer.text.contains("answers no") || answer.text.contains("runs no") {
            unanswered.push(tool.name);
        }
        if tool.writes && !answer.failed {
            took_nothing.push(tool.name);
        }
    }
    assert!(
        took_nothing.is_empty(),
        "wrote with no argument: {took_nothing:?}"
    );
    assert!(unanswered.is_empty(), "{unanswered:?}");
}

#[test]
fn a_note_the_agent_leaves_stands_on_its_own_line() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);
    state
        .agent
        .auto_approve(&groove_types::SessionId::new(&id), true);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "create_annotation",
        json!({ "worktree_id": worktree, "path": "a.txt", "line": 2, "content": "issue: this" }),
    );
    assert!(!answer.failed, "{}", answer.text);
    until(&spawner, &services, &mut state, |s| {
        !s.delivery
            .notes_of(&groove_types::SessionId::new(&id))
            .unwrap_or_default()
            .is_empty()
    });
    let note = &state
        .delivery
        .notes_of(&groove_types::SessionId::new(&id))
        .unwrap_or_default()[0];
    assert_eq!(note.file_path, "a.txt");
    assert_eq!(
        note.start_line, 1,
        "the agent counts from one, the row from zero"
    );
    assert_eq!(note.author, "agent");

    let again = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "create_annotation",
        json!({ "worktree_id": worktree, "path": "a.txt", "line": 2, "content": "issue: again" }),
    );
    assert!(again.failed, "one note a line: {}", again.text);
}

#[test]
fn a_note_is_written_again_and_resolved_by_its_id() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);
    state
        .agent
        .auto_approve(&groove_types::SessionId::new(&id), true);
    asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "create_annotation",
        json!({ "worktree_id": worktree, "path": "a.txt", "line": 1, "content": "issue: one" }),
    );
    until(&spawner, &services, &mut state, |s| {
        !s.delivery
            .notes_of(&groove_types::SessionId::new(&id))
            .unwrap_or_default()
            .is_empty()
    });
    let note = state
        .delivery
        .notes_of(&groove_types::SessionId::new(&id))
        .unwrap_or_default()[0]
        .id
        .as_str()
        .to_string();

    let wrote = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "update_annotation",
        json!({ "id": note, "content": "issue: two" }),
    );
    assert!(!wrote.failed, "{}", wrote.text);
    until(&spawner, &services, &mut state, |s| {
        s.delivery
            .notes_of(&groove_types::SessionId::new(&id))
            .unwrap_or_default()
            .first()
            .is_some_and(|one| one.content == "issue: two")
    });

    let done = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "resolve_annotation",
        json!({ "id": note }),
    );
    assert!(!done.failed, "{}", done.text);
    until(&spawner, &services, &mut state, |s| {
        s.delivery
            .notes_of(&groove_types::SessionId::new(&id))
            .unwrap_or_default()
            .first()
            .is_some_and(|one| one.status == groove_types::AnnotationStatus::Resolved)
    });
}

#[test]
fn the_agent_cuts_a_second_worktree_and_gets_its_id() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let dir = fixture::worktree(&mut state, &services, &spawner);
    assert!(!dir.is_empty());
    let id = state.session.selected.clone().expect("a session");
    state
        .agent
        .auto_approve(&groove_types::SessionId::new(id.as_str()), true);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        id.as_str(),
        "add_task_worktree",
        json!({ "branch": "explorer/second", "target_branch": "main" }),
    );
    assert!(!answer.failed, "{}", answer.text);
    assert!(answer.text.contains("worktree_id"), "{}", answer.text);
    let open = state.session.get(&id).expect("the session");
    assert_eq!(open.repos.len(), 1, "the repo it already had");
    assert_eq!(open.worktrees.len(), 2, "and a second worktree on it");
}

#[test]
fn the_agent_and_the_surface_commit_through_the_same_function() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let (id, worktree, dir) = changed(&mut state, &services, &spawner);
    state
        .agent
        .auto_approve(&groove_types::SessionId::new(&id), true);
    stage(&dir);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "git_commit",
        json!({ "worktree_id": worktree, "message": "fix: from the agent" }),
    );
    assert!(!answer.failed, "{}", answer.text);
    until(&spawner, &services, &mut state, |s| {
        s.session
            .feed
            .iter()
            .any(|one| one.subject == "fix: from the agent")
    });
    let line = state
        .session
        .feed
        .iter()
        .find(|one| one.subject == "fix: from the agent")
        .expect("the commit's own line");
    assert_eq!(line.kind, groove_types::TimelineKind::Commit);
}

#[test]
fn a_write_the_forge_refuses_is_answered_and_not_left_to_the_feed() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);
    state
        .agent
        .auto_approve(&groove_types::SessionId::new(&id), true);

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "update_mr",
        json!({ "worktree_id": worktree, "title": "feat: nothing" }),
    );
    assert!(answer.failed, "{}", answer.text);
    assert!(
        answer.text.contains("no open merge request"),
        "{}",
        answer.text
    );
    assert!(
        state.errors.is_empty(),
        "the agent hears it, the feed does not"
    );
}

#[test]
fn a_push_waits_with_its_branch_and_the_commits_it_sends() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let (id, worktree, dir) = changed(&mut state, &services, &spawner);
    stage(&dir);
    sh(
        std::path::Path::new(&dir),
        &["commit", "-m", "feat: two lines\n\nthe body"],
    );

    let (reply, _answered) = Reply::new();
    let call = Call {
        session: id.clone(),
        tool: "git_push".into(),
        arguments: json!({ "worktree_id": worktree }),
        reply,
    };
    crate::tools::answer(&mut state, &services, &spawner, call);
    until(&spawner, &services, &mut state, |s| {
        !waiting(s, &id).is_empty()
    });
    let ask = &waiting(&state, &id)[0];
    let mut lines = ask.text.lines();
    assert!(
        lines.next().is_some_and(|one| one.starts_with("explorer/")),
        "{}",
        ask.text
    );
    assert_eq!(lines.next(), Some(""));
    let sent: Vec<&str> = lines.collect();
    assert_eq!(sent.len(), 1, "only the commit origin lacks: {}", ask.text);
    assert!(sent[0].ends_with(" feat: two lines"), "{}", ask.text);
}
