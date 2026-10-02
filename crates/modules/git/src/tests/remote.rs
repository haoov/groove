use crate::Error;
use crate::tests::fixture::{Fixture, sh};

#[tokio::test]
async fn origin_lists_its_heads() {
    let fx = Fixture::new();
    assert_eq!(
        fx.git().remote_heads().await.unwrap(),
        ["main", "release/1.0"]
    );
}

#[tokio::test]
async fn push_then_pull_round_trip_and_a_diverged_pull_is_typed() {
    let fx = Fixture::new();
    let git = fx.git();
    git.push("main").await.unwrap();
    assert_eq!(
        sh(&fx.work, &["rev-parse", "origin/main"]),
        sh(&fx.work, &["rev-parse", "HEAD"])
    );

    let other = fx.root.path().join("other");
    crate::Git::clone(fx.origin.to_str().unwrap(), &other)
        .await
        .unwrap();
    sh(&other, &["config", "user.email", "t@t"]);
    sh(&other, &["config", "user.name", "T"]);
    std::fs::write(other.join("b.txt"), "b\n").unwrap();
    sh(&other, &["add", "."]);
    sh(&other, &["commit", "-m", "from other"]);
    sh(&other, &["push", "origin", "main"]);

    git.fetch(&[]).await.unwrap();
    git.pull().await.unwrap();
    assert!(fx.work.join("b.txt").exists(), "fast-forwarded");

    std::fs::write(fx.work.join("c.txt"), "c\n").unwrap();
    sh(&fx.work, &["add", "."]);
    sh(&fx.work, &["commit", "-m", "local"]);
    std::fs::write(other.join("d.txt"), "d\n").unwrap();
    sh(&other, &["add", "."]);
    sh(&other, &["commit", "-m", "remote"]);
    sh(&other, &["push", "origin", "main"]);
    let err = git.pull().await.unwrap_err();
    assert!(
        matches!(err, Error::Diverged { ref branch } if branch == "main"),
        "{err}"
    );
}

#[tokio::test]
async fn a_missing_origin_is_an_error_not_an_empty_list() {
    let fx = Fixture::new();
    sh(
        &fx.work,
        &["remote", "set-url", "origin", "/nonexistent/origin.git"],
    );
    assert!(matches!(
        fx.git().remote_heads().await,
        Err(Error::Failed { .. })
    ));
}

#[tokio::test]
async fn a_followed_copy_holds_the_branch_head_whatever_was_done_to_it() {
    let fx = Fixture::new();
    fx.git().push("main").await.unwrap();
    let copy = fx.root.path().join("copy");
    let url = format!("file://{}", fx.origin.display());
    let git = crate::Git::clone_head(&url, &copy, "main").await.unwrap();
    assert_eq!(
        std::fs::read_to_string(copy.join("a.txt")).unwrap(),
        "two\n"
    );

    std::fs::write(fx.work.join("a.txt"), "three\n").unwrap();
    sh(&fx.work, &["commit", "-am", "third"]);
    fx.git().push("main").await.unwrap();
    std::fs::write(copy.join("a.txt"), "edited here\n").unwrap();
    std::fs::write(copy.join("stray.txt"), "left\n").unwrap();
    git.follow("main").await.unwrap();
    assert_eq!(
        std::fs::read_to_string(copy.join("a.txt")).unwrap(),
        "three\n"
    );
    assert!(
        !copy.join("stray.txt").exists(),
        "nothing of its own is kept"
    );
}

#[tokio::test]
async fn a_branch_origin_does_not_hold_is_an_error() {
    let fx = Fixture::new();
    let url = format!("file://{}", fx.origin.display());
    let copy = fx.root.path().join("copy");
    assert!(crate::Git::clone_head(&url, &copy, "nope").await.is_err());
}
