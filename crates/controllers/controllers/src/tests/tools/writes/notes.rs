//! The notes the agent leaves, writes again and resolves.

use super::*;

#[test]
fn a_note_the_agent_leaves_stands_on_its_own_line() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);
    crate::tests::fixture::auto_approve(&mut state, &groove_types::SessionId::new(&id));

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
    crate::tests::fixture::auto_approve(&mut state, &groove_types::SessionId::new(&id));
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
            .is_some_and(|one| one.content == "issue (agent): two")
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
