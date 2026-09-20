use crate::tests::fixture::{Fixture, sh};
use crate::{Error, WorktreeSpec};

#[tokio::test]
async fn close_refuses_lost_work_then_removes_dir_row_branch_and_empty_parents() {
    let fx = Fixture::new().await;
    let repo = fx.repo().await;
    let wt = fx
        .pool
        .provision(&fx.session, &repo, &WorktreeSpec::default(), None)
        .await
        .unwrap()
        .worktree;
    let path = std::path::PathBuf::from(&wt.path);
    std::fs::write(path.join("a.txt"), "dirty\n").unwrap();
    assert!(matches!(
        fx.pool.close(&wt.id, false).await,
        Err(Error::Dirty)
    ));

    sh(&path, &["commit", "-am", "unpushed"]);
    let err = fx.pool.close(&wt.id, false).await.unwrap_err();
    assert!(matches!(err, Error::Unpushed { ahead: 1, .. }), "{err}");

    let closed = fx.pool.close(&wt.id, true).await.unwrap();
    assert_eq!(closed.id, wt.id);
    assert!(!path.exists());
    let session_dir = fx.root.path().join("worktrees/explorer-ab12cd34");
    assert!(!session_dir.join("mayo").exists(), "empty parents go");
    assert!(session_dir.exists(), "the session dir stays");
    assert!(
        fx.pool
            .worktrees_of(&fx.session.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!sh(&fx.clone, &["worktree", "list"]).contains("try-sqlite-vacuum"));
    assert!(
        !sh(
            &fx.clone,
            &["branch", "--list", "explorer/try-sqlite-vacuum"]
        )
        .contains("try-sqlite-vacuum"),
        "the local branch goes too"
    );
}

#[tokio::test]
async fn status_counts_changes_and_commits_ahead_of_the_base() {
    let fx = Fixture::new().await;
    let repo = fx.repo().await;
    let wt = fx
        .pool
        .provision(&fx.session, &repo, &WorktreeSpec::default(), None)
        .await
        .unwrap()
        .worktree;
    let path = std::path::Path::new(&wt.path);
    assert_eq!(fx.pool.status(&wt).await.unwrap(), Default::default());
    std::fs::write(path.join("a.txt"), "two\n").unwrap();
    std::fs::write(path.join("b.txt"), "b\n").unwrap();
    sh(path, &["add", "b.txt"]);
    let status = fx.pool.status(&wt).await.unwrap();
    assert_eq!((status.modified, status.staged), (1, 1));
    sh(path, &["commit", "-am", "work"]);
    let status = fx.pool.status(&wt).await.unwrap();
    assert_eq!(
        (status.ahead, status.behind),
        (1, 0),
        "never pushed: ahead of the base"
    );
}

#[tokio::test]
async fn cleanup_removes_every_worktree_and_the_session_dir() {
    let fx = Fixture::new().await;
    let repo = fx.repo().await;
    fx.pool
        .provision(&fx.session, &repo, &WorktreeSpec::default(), None)
        .await
        .unwrap();
    let second = WorktreeSpec {
        branch: Some("explorer/second".into()),
        ..Default::default()
    };
    fx.pool
        .provision(&fx.session, &repo, &second, None)
        .await
        .unwrap();
    assert_eq!(fx.pool.worktrees_of(&fx.session.id).await.unwrap().len(), 2);
    fx.pool.cleanup_session(&fx.session.id, true).await.unwrap();
    assert!(!fx.root.path().join("worktrees/explorer-ab12cd34").exists());
    assert_eq!(
        sh(&fx.clone, &["branch", "--list", "explorer/*"]),
        "",
        "every session branch is gone"
    );
    assert!(
        fx.pool
            .worktrees_of(&fx.session.id)
            .await
            .unwrap()
            .is_empty()
    );
}
