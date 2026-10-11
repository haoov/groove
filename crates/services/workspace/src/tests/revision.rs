use groove_types::{RepoId, SessionId, Timestamp, Worktree, WorktreeId};

use crate::{Revision, Step};

fn worktree(id: &str, repo: &str) -> Worktree {
    Worktree {
        id: WorktreeId::new(id),
        session: SessionId::new("s"),
        repo: RepoId::new(repo),
        branch: "main".into(),
        path: format!("/w/{id}"),
        base_ref: None,
        created_at: Timestamp::new(0),
    }
}

fn asked(sha: Option<&str>) -> Revision {
    Revision {
        session: SessionId::new("s"),
        slug: "gitlab.example.com/g/deploy".into(),
        sha: sha.map(String::from),
        reading: false,
    }
}

#[test]
fn a_revision_selects_its_worktree_then_reads_its_commit_once_read() {
    let (here, other) = (SessionId::new("s"), SessionId::new("t"));
    let (api, deploy) = (
        worktree("a", "gitlab.example.com/g/api"),
        worktree("d", "gitlab.example.com/g/deploy"),
    );
    let all = [api.clone(), deploy.clone()];
    let pending = asked(Some("abc"));
    assert_eq!(
        pending.step(&other, Some(&api), &all, false),
        Step::Drop,
        "another session"
    );
    assert_eq!(
        pending.step(&here, Some(&api), &all, false),
        Step::Select(deploy.id.clone())
    );
    assert_eq!(
        pending.step(&here, Some(&deploy), &all, true),
        Step::Wait,
        "not read yet"
    );
    assert_eq!(
        pending.step(&here, Some(&deploy), &all, false),
        Step::Read("abc".into())
    );
    assert_eq!(
        asked(None).step(&here, Some(&deploy), &all, false),
        Step::Drop,
        "nothing to show"
    );
    assert_eq!(
        pending.step(&here, Some(&api), &[api.clone()], false),
        Step::Wait,
        "not added yet"
    );
}
