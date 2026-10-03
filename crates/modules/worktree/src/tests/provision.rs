use groove_types::{ExternalId, Session, SessionId, SessionKind, Timestamp};

use crate::tests::fixture::{Fixture, sh};
use crate::{Error, WorktreeSpec};

#[tokio::test]
async fn a_default_explorer_worktree_is_cut_from_the_default_branch() {
    let fx = Fixture::new().await;
    let repo = fx.repo().await;
    let done = fx
        .pool
        .provision(&fx.session, &repo, &WorktreeSpec::default(), None)
        .await
        .unwrap();
    let wt = &done.worktree;
    assert_eq!(wt.branch, "explorer/ab12cd34");
    assert_eq!(
        wt.path,
        fx.root
            .path()
            .join("worktrees/explorer-ab12cd34/mayo/explorer/ab12cd34")
            .to_string_lossy()
    );
    assert!(wt.base_ref.is_none());
    assert!(!done.adopted);
    assert!(done.notes.is_empty(), "{:?}", done.notes);
    assert_eq!(
        sh(
            std::path::Path::new(&wt.path),
            &["rev-parse", "--abbrev-ref", "HEAD"]
        ),
        "explorer/ab12cd34"
    );
    assert_eq!(
        sh(std::path::Path::new(&wt.path), &["rev-parse", "HEAD"]),
        sh(&fx.clone, &["rev-parse", "origin/main"])
    );
    assert_eq!(
        fx.pool.worktrees_of(&fx.session.id).await.unwrap(),
        vec![wt.clone()]
    );

    let again = fx
        .pool
        .provision(&fx.session, &repo, &WorktreeSpec::default(), None)
        .await
        .unwrap();
    assert_eq!(again.worktree.id, wt.id, "idempotent on the same branch");
}

#[tokio::test]
async fn a_target_pins_the_base_and_an_unknown_target_lists_what_exists() {
    let fx = Fixture::new().await;
    let repo = fx.repo().await;
    let spec = WorktreeSpec {
        branch: Some("fix/x".into()),
        target: Some("release/1.0".into()),
        track_remote: None,
    };
    let done = fx
        .pool
        .provision(&fx.session, &repo, &spec, None)
        .await
        .unwrap();
    assert_eq!(done.worktree.base_ref.as_deref(), Some("release/1.0"));
    assert_eq!(
        sh(
            std::path::Path::new(&done.worktree.path),
            &["rev-parse", "HEAD"]
        ),
        sh(&fx.clone, &["rev-parse", "origin/release/1.0"])
    );

    let bad = WorktreeSpec {
        branch: Some("fix/y".into()),
        target: Some("nope".into()),
        track_remote: None,
    };
    let err = fx
        .pool
        .provision(&fx.session, &repo, &bad, None)
        .await
        .unwrap_err();
    assert!(
        matches!(&err, Error::NoTarget { available, .. } if available == &["main", "release/1.0"]),
        "{err}"
    );
}

#[tokio::test]
async fn an_existing_branch_of_the_session_s_own_is_adopted() {
    let fx = Fixture::new().await;
    let repo = fx.repo().await;
    sh(&fx.clone, &["branch", "explorer/ab12cd34", "origin/main"]);
    let done = fx
        .pool
        .provision(&fx.session, &repo, &WorktreeSpec::default(), None)
        .await
        .unwrap();
    assert!(done.adopted, "the explorer's own branch is its own");
    assert!(done.notes[0].contains("continuing on the existing branch"));

    let task = Session {
        id: SessionId::new("gh-groove-50"),
        title: "Fix the parser".into(),
        kind: SessionKind::Task {
            external_id: ExternalId::new("page"),
        },
        created_at: Timestamp::new(0),
    };
    sqlx::query("INSERT INTO sessions (id, kind, title, external_id, created_at) VALUES ('gh-groove-50', 'task', 'Fix the parser', 'page', 0)")
        .execute(fx.pool.db.pool())
        .await
        .unwrap();
    sh(
        &fx.clone,
        &["branch", "fix/fix-the-parser-gh-groove-50", "origin/main"],
    );
    let own = fx
        .pool
        .provision(&task, &repo, &WorktreeSpec::default(), None)
        .await
        .unwrap();
    assert!(own.adopted, "a branch naming the session is the session's");
}

#[tokio::test]
async fn a_review_worktree_tracks_the_remote_branch() {
    let fx = Fixture::new().await;
    let repo = fx.repo().await;
    let spec = WorktreeSpec {
        branch: Some("release/1.0".into()),
        target: Some("main".into()),
        track_remote: Some("release/1.0".into()),
    };
    let done = fx
        .pool
        .provision(&fx.session, &repo, &spec, None)
        .await
        .unwrap();
    let path = std::path::Path::new(&done.worktree.path);
    assert_eq!(
        sh(
            path,
            &["rev-parse", "--abbrev-ref", "release/1.0@{upstream}"]
        ),
        "origin/release/1.0"
    );
    assert_eq!(done.worktree.base_ref.as_deref(), Some("main"));
}

#[tokio::test]
async fn a_bad_branch_name_never_reaches_git() {
    let fx = Fixture::new().await;
    let repo = fx.repo().await;
    let spec = WorktreeSpec {
        branch: Some("bad name".into()),
        ..Default::default()
    };
    assert!(matches!(
        fx.pool.provision(&fx.session, &repo, &spec, None).await,
        Err(Error::InvalidBranch { .. })
    ));
}
