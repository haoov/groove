use groove_types::{Session, SessionId, SessionKind, Timestamp};

use crate::WorktreeSpec;
use crate::tests::fixture::{Fixture, sh};

fn task_session() -> Session {
    Session {
        id: SessionId::new("gh-g-7"),
        kind: SessionKind::Task {
            external_id: groove_types::ExternalId::new("gh:g#7"),
        },
        title: "Fix the thing".into(),
        created_at: Timestamp::now(),
    }
}

#[tokio::test]
async fn an_explorer_s_worktree_lands_under_the_task_on_the_task_s_branch_and_can_go_back() {
    let fx = Fixture::new().await;
    let repo = fx.repo().await;
    let spec = WorktreeSpec::default();
    let cut = fx.pool.provision(&fx.session, &repo, &spec, None).await;
    let worktree = cut.unwrap().worktree;
    let to = task_session();
    let relocated = fx
        .pool
        .relocate(&fx.session.id, &to, std::slice::from_ref(&worktree), "g-7")
        .await
        .unwrap();
    let (_, branch, path) = relocated.moves().next().unwrap();
    let (branch, path) = (branch.to_string(), path);
    assert!(path.contains("worktrees/gh-g-7/mayo/"), "{path}");
    let head = sh(
        std::path::Path::new(&path),
        &["rev-parse", "--abbrev-ref", "HEAD"],
    );
    assert_eq!(head.trim(), branch);
    relocated.put_back().await;
    assert!(std::path::Path::new(&worktree.path).is_dir());
    let head = sh(
        std::path::Path::new(&worktree.path),
        &["rev-parse", "--abbrev-ref", "HEAD"],
    );
    assert_eq!(head.trim(), worktree.branch);
}

#[tokio::test]
async fn a_task_branch_the_clone_already_holds_takes_a_number_and_the_move_goes_on() {
    let fx = Fixture::new().await;
    let repo = fx.repo().await;
    let spec = WorktreeSpec::default();
    let cut = fx.pool.provision(&fx.session, &repo, &spec, None).await;
    let worktree = cut.unwrap().worktree;
    let to = task_session();
    let wanted = crate::naming::session_branch(&to, Some("g-7"));
    sh(std::path::Path::new(&worktree.path), &["branch", &wanted]);
    let relocated = fx
        .pool
        .relocate(&fx.session.id, &to, std::slice::from_ref(&worktree), "g-7")
        .await
        .expect("the move goes on");
    let (_, branch, _) = relocated.moves().next().unwrap();
    assert_eq!(branch, format!("{wanted}-2"));
}
