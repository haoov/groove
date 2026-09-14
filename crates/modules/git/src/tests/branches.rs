use crate::Error;
use crate::tests::fixture::{Fixture, sh};

#[tokio::test]
async fn a_worktree_is_cut_on_a_new_branch_and_refuses_a_second_time() {
    let fx = Fixture::new();
    let git = fx.git();
    git.branch_create("feat/x", "origin/release/1.0")
        .await
        .unwrap();
    let wt = fx.root.path().join("wt/feat/x");
    git.worktree_add(&wt, "feat/x").await.unwrap();
    assert_eq!(
        crate::Git::at(&wt).current_branch().await.unwrap(),
        "feat/x"
    );
    assert_eq!(
        sh(&wt, &["rev-parse", "HEAD"]),
        sh(&fx.work, &["rev-parse", "origin/release/1.0"])
    );

    let err = git.worktree_add(&wt, "feat/x").await.unwrap_err();
    assert!(matches!(err, Error::AlreadyExists { .. }), "{err}");
    assert_eq!(
        groove_types::Error::from(err).kind,
        groove_types::ErrorKind::Conflict
    );
}

#[tokio::test]
async fn a_review_worktree_tracks_the_remote_branch() {
    let fx = Fixture::new();
    let wt = fx.root.path().join("wt/review");
    fx.git()
        .worktree_add_tracking(&wt, "release/1.0", "release/1.0")
        .await
        .unwrap();
    assert_eq!(
        sh(
            &wt,
            &["rev-parse", "--abbrev-ref", "release/1.0@{upstream}"]
        ),
        "origin/release/1.0"
    );
}

#[tokio::test]
async fn switch_creates_or_checks_out_and_prune_forgets_a_removed_worktree() {
    let fx = Fixture::new();
    let git = fx.git();
    let wt = fx.root.path().join("wt/tmp");
    git.branch_create("tmp", "HEAD").await.unwrap();
    git.worktree_add(&wt, "tmp").await.unwrap();
    let inner = crate::Git::at(&wt);
    inner.switch("tmp-2", true).await.unwrap();
    assert_eq!(inner.current_branch().await.unwrap(), "tmp-2");
    inner.switch("tmp", false).await.unwrap();
    assert_eq!(inner.current_branch().await.unwrap(), "tmp");

    std::fs::remove_dir_all(&wt).unwrap();
    assert!(sh(&fx.work, &["worktree", "list"]).contains("wt/tmp"));
    git.worktree_prune().await.unwrap();
    assert!(!sh(&fx.work, &["worktree", "list"]).contains("wt/tmp"));
    git.branch_delete("tmp-2").await.unwrap();
    assert!(!sh(&fx.work, &["branch", "--list", "tmp-2"]).contains("tmp-2"));
    assert!(git.branch_delete("tmp-2").await.is_err(), "already gone");
}
