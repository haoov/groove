use crate::tests::fixture::{Fixture, sh};

/// The paths the status reports, with whether the index holds them.
async fn state(git: &crate::Git) -> Vec<(String, bool)> {
    let mut rows: Vec<(String, bool)> = git
        .status()
        .await
        .unwrap()
        .into_iter()
        .map(|change| (change.path.clone(), change.is_staged()))
        .collect();
    rows.sort();
    rows
}

#[tokio::test]
async fn a_commit_takes_the_index_and_leaves_the_rest() {
    let fx = Fixture::new();
    let git = fx.git();
    sh(&fx.work, &["config", "user.email", "t@t"]);
    sh(&fx.work, &["config", "user.name", "T"]);
    sh(&fx.work, &["config", "commit.gpgsign", "false"]);
    std::fs::write(fx.work.join("a.txt"), "three\n").unwrap();
    std::fs::write(fx.work.join("b.txt"), "kept\n").unwrap();
    sh(&fx.work, &["add", "a.txt"]);

    git.commit("fix(a): three").await.unwrap();
    assert_eq!(sh(&fx.work, &["log", "-1", "--pretty=%s"]), "fix(a): three");
    assert_eq!(
        state(&git).await,
        [("b.txt".to_string(), false)],
        "what was not staged is still waiting"
    );
}

#[tokio::test]
async fn a_commit_with_nothing_staged_is_an_error() {
    let fx = Fixture::new();
    let git = fx.git();
    sh(&fx.work, &["config", "user.email", "t@t"]);
    sh(&fx.work, &["config", "user.name", "T"]);
    sh(&fx.work, &["config", "commit.gpgsign", "false"]);
    let refused = git.commit("chore: nothing").await;
    assert!(refused.is_err(), "git has nothing to commit");
}
