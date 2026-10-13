//! A write acts on its own session's worktrees and notes, never another's.

use serde_json::json;

use super::{changed, stage, waited};
use crate::tests::fixture::{self, sh};

#[test]
fn a_write_naming_another_session_s_worktree_is_refused() {
    let (home, spawner, services, mut state) = fixture::fresh();
    fixture::pooled_clone(home.path());
    let (_theirs, worktree, dir) = changed(&mut state, &services, &spawner);
    stage(&dir);
    let ours = fixture::explorer(&mut state, &services, &spawner);
    fixture::auto_approve(&mut state, &ours);

    let answer = waited(
        &mut state,
        &services,
        &spawner,
        ours.as_str(),
        "git_commit",
        json!({ "worktree_id": worktree, "message": "fix: not mine" }),
    )
    .expect("an answer");
    assert!(answer.failed, "{}", answer.text);
    assert_eq!(answer.text, crate::tools::NO_WORKTREE);
    let log = sh(std::path::Path::new(&dir), &["log", "-1", "--pretty=%s"]);
    assert!(!log.contains("not mine"), "{log}");
}
