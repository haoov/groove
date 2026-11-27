//! What the agent says on the MR's threads.

use super::*;

#[test]
fn a_comment_on_a_path_needs_a_line() {
    let (home, spawner, services, mut state) = crate::tests::fixture::fresh();
    fixture::pooled_clone(home.path());
    let (id, worktree, _) = changed(&mut state, &services, &spawner);
    crate::tests::fixture::auto_approve(&mut state, &groove_types::SessionId::new(&id));

    let answer = asked(
        &mut state,
        &services,
        &spawner,
        &id,
        "comment_mr",
        json!({ "worktree_id": worktree, "path": "a.txt", "body": "issue: this" }),
    );
    assert!(answer.failed, "{}", answer.text);
    assert!(answer.text.contains("needs a line"), "{}", answer.text);
}
