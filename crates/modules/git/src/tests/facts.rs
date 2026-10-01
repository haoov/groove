use crate::Error;
use crate::tests::fixture::{Fixture, sh};

#[tokio::test]
async fn a_clone_knows_itself() {
    let fx = Fixture::new();
    let git = fx.git();
    assert!(git.is_repository().await.unwrap());
    assert!(git.has_remote("origin").await.unwrap());
    assert!(!git.has_remote("upstream").await.unwrap());
    assert!(
        !crate::Git::at(fx.root.path())
            .is_repository()
            .await
            .unwrap()
    );
    assert_eq!(git.current_branch().await.unwrap(), "main");
    assert!(git.ref_exists("origin/release/1.0").await.unwrap());
    assert!(!git.ref_exists("origin/nope").await.unwrap());
    assert!(git.version().await.unwrap().starts_with("git version"));
    let base = git.merge_base("origin/main", "HEAD").await.unwrap();
    assert_eq!(base, sh(&fx.work, &["rev-parse", "origin/main"]));
}

#[tokio::test]
async fn the_remote_url_is_a_pool_slug() {
    let fx = Fixture::new();
    sh(
        &fx.work,
        &[
            "remote",
            "set-url",
            "origin",
            "git@gitlab.example.com:wiremind/devops/groove.git",
        ],
    );
    let url = fx.git().remote_url("origin").await.unwrap();
    assert_eq!(url.slug(), "gitlab.example.com/wiremind/devops/groove");
}

#[tokio::test]
async fn the_default_branch_comes_from_the_symref_then_from_origin() {
    let fx = Fixture::new();
    let git = fx.git();
    assert_eq!(git.default_branch().await.unwrap().as_deref(), Some("main"));
    sh(&fx.work, &["remote", "set-head", "origin", "--delete"]);
    assert_eq!(
        git.default_branch().await.unwrap().as_deref(),
        Some("main"),
        "asked of origin"
    );
    assert!(
        !fx.work.join(".git/refs/remotes/origin/HEAD").exists(),
        "nothing written back"
    );
}

#[tokio::test]
async fn the_base_ref_prefers_the_pin_then_falls_back() {
    let fx = Fixture::new();
    let git = fx.git();
    assert_eq!(
        git.base_ref(Some("release/1.0")).await.unwrap(),
        "origin/release/1.0"
    );
    assert_eq!(
        git.base_ref(Some("no-such-branch")).await.unwrap(),
        "origin/HEAD"
    );
    assert_eq!(git.base_ref(Some("")).await.unwrap(), "origin/HEAD");
    sh(&fx.work, &["remote", "set-head", "origin", "--delete"]);
    assert_eq!(git.base_ref(None).await.unwrap(), "origin/main");
    sh(&fx.work, &["remote", "remove", "origin"]);
    let err = git.base_ref(Some("main")).await.unwrap_err();
    assert!(
        matches!(&err, Error::NoBase { tried, .. } if tried.len() == 4),
        "{err}"
    );
    assert_eq!(
        groove_types::Error::from(err).kind,
        groove_types::ErrorKind::Git
    );
}

#[tokio::test]
async fn a_remote_branch_of_the_same_name_the_branch_does_not_track_is_not_its_own() {
    let fx = Fixture::new();
    let git = fx.git();
    sh(
        &fx.work,
        &["push", "-q", "origin", "HEAD~1:refs/heads/shared"],
    );
    sh(&fx.work, &["fetch", "-q", "origin"]);
    sh(
        &fx.work,
        &["checkout", "-q", "-b", "shared", "--track", "origin/main"],
    );
    assert_eq!(
        git.pushed_point("shared", None).await.unwrap(),
        "origin/HEAD",
        "it tracks main: its base, as git status says"
    );
    assert_eq!(git.ahead_behind("shared").await.unwrap(), None);
}

#[tokio::test]
async fn origin_stands_at_the_branch_once_pushed_and_at_its_base_before() {
    let fx = Fixture::new();
    let git = fx.git();
    sh(&fx.work, &["checkout", "-q", "-b", "fix/one"]);
    assert_eq!(
        git.pushed_point("fix/one", Some("release/1.0"))
            .await
            .unwrap(),
        "origin/release/1.0",
        "never pushed: its base"
    );
    sh(&fx.work, &["push", "-q", "origin", "fix/one"]);
    assert_eq!(
        git.pushed_point("fix/one", Some("release/1.0"))
            .await
            .unwrap(),
        "origin/fix/one"
    );
}
