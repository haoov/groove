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
}
