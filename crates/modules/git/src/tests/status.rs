use crate::tests::fixture::{Fixture, sh};

#[tokio::test]
async fn status_rows_and_ahead_behind() {
    let fx = Fixture::new();
    let git = fx.git();
    assert!(git.status().await.unwrap().is_empty());
    std::fs::write(fx.work.join("a.txt"), "three\n").unwrap();
    std::fs::write(fx.work.join("new.txt"), "n\n").unwrap();
    sh(&fx.work, &["add", "new.txt"]);
    let rows = git.status().await.unwrap();
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .any(|c| c.path == "a.txt" && c.is_modified() && !c.is_staged())
    );
    assert!(rows.iter().any(|c| c.path == "new.txt" && c.is_staged()));

    assert_eq!(
        git.ahead_behind("main").await.unwrap(),
        Some((1, 0)),
        "the unpushed second commit"
    );
    assert_eq!(git.ahead_behind("never-pushed").await.unwrap(), None);
    assert_eq!(git.commits_since("origin/main").await.unwrap(), 1);
    assert_eq!(git.commits_since("HEAD").await.unwrap(), 0);
}

#[tokio::test]
async fn blobs_reads_every_path_at_a_ref() {
    let fx = Fixture::new();
    std::fs::write(fx.work.join("b.txt"), "second\n").unwrap();
    sh(&fx.work, &["add", "b.txt"]);
    sh(&fx.work, &["commit", "-m", "b"]);
    std::fs::write(fx.work.join("b.txt"), "changed\n").unwrap();
    let paths = [
        "a.txt".to_string(),
        "b.txt".to_string(),
        "no.txt".to_string(),
    ];
    let found = fx.git().blobs("HEAD", &paths).await.unwrap();
    assert_eq!(found.get("b.txt").map(String::as_str), Some("second\n"));
    assert!(found.contains_key("a.txt"), "the committed file: {found:?}");
    assert!(
        !found.contains_key("no.txt"),
        "a path HEAD has no object for"
    );
}
