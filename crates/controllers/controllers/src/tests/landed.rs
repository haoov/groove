//! A worktree whose MR merged: its commits are on the forge, so closing it loses none.

use groove_types::{Forge, Mr, MrId, MrState, RepoId};

use super::fixture::{self, pooled_clone, sh};
use super::worktrees::{REPO, close, explorer, second_worktree};

#[test]
fn a_merged_mr_s_worktree_closes_with_commits_origin_lacks_but_not_with_changes() {
    let (home, spawner, services, mut state) = fixture::fresh();
    pooled_clone(home.path());
    let id = explorer(&mut state, &services, &spawner);
    let repo = RepoId::new(REPO);
    let second = second_worktree(&mut state, &services, &spawner, &id, &repo);
    let path = std::path::Path::new(&second.path);
    std::fs::write(path.join("a.txt"), "squashed\n").expect("a file");
    sh(path, &["add", "a.txt"]);
    sh(path, &["commit", "-m", "squashed upstream"]);
    close(&mut state, &services, &spawner, &id, &second.id, false);
    assert_eq!(state.errors.len(), 1, "an open branch keeps its commits");
    state.errors.clear();
    state.delivery.remembered(vec![Mr {
        id: MrId::new("mr-1"),
        worktree: second.id.clone(),
        forge: Forge::Gitlab,
        remote_id: "1".into(),
        url: "https://gitlab.example.com/g/mayo/-/merge_requests/1".into(),
        state: MrState::Merged,
    }]);
    std::fs::write(path.join("a.txt"), "edited after\n").expect("an edit");
    close(&mut state, &services, &spawner, &id, &second.id, false);
    assert_eq!(
        state.errors.len(),
        1,
        "a merged branch still keeps its changes"
    );
    state.errors.clear();
    sh(path, &["checkout", "--", "a.txt"]);
    close(&mut state, &services, &spawner, &id, &second.id, false);
    assert!(state.errors.is_empty(), "{:?}", state.errors);
    let open = state.session.get(&id).expect("the session");
    assert!(open.worktrees.iter().all(|one| one.id != second.id));
}
