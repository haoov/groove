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
async fn a_change_is_staged_and_taken_back_out() {
    let fx = Fixture::new();
    let git = fx.git();
    std::fs::write(fx.work.join("a.txt"), "three\n").unwrap();
    std::fs::write(fx.work.join("new.txt"), "n\n").unwrap();

    git.stage(&["a.txt".into(), "new.txt".into()])
        .await
        .unwrap();
    assert_eq!(
        state(&git).await,
        [("a.txt".to_string(), true), ("new.txt".to_string(), true)]
    );

    git.unstage(&["new.txt".into()]).await.unwrap();
    let rows = state(&git).await;
    assert_eq!(rows[0], ("a.txt".to_string(), true), "still staged");
    assert_eq!(rows[1], ("new.txt".to_string(), false), "untracked again");
    assert!(fx.work.join("new.txt").exists(), "and still on disk");
}

#[tokio::test]
async fn discarding_a_tracked_file_puts_it_back_and_an_untracked_one_goes() {
    let fx = Fixture::new();
    let git = fx.git();
    let before = std::fs::read_to_string(fx.work.join("a.txt")).unwrap();
    std::fs::write(fx.work.join("a.txt"), "three\n").unwrap();
    std::fs::write(fx.work.join("new.txt"), "n\n").unwrap();
    git.stage(&["a.txt".into()]).await.unwrap();

    git.discard(&["a.txt".into(), "new.txt".into()])
        .await
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(fx.work.join("a.txt")).unwrap(),
        before,
        "the tracked file is what HEAD has"
    );
    assert!(
        !fx.work.join("new.txt").exists(),
        "and the untracked one has nowhere to go back to"
    );
    assert!(state(&git).await.is_empty(), "nothing changed any more");
}

#[tokio::test]
async fn discarding_a_staged_new_file_takes_it_out_of_the_index_and_the_tree() {
    let fx = Fixture::new();
    let git = fx.git();
    std::fs::write(fx.work.join("new.txt"), "n\n").unwrap();
    git.stage(&["new.txt".into()]).await.unwrap();
    assert_eq!(state(&git).await, [("new.txt".to_string(), true)]);

    git.discard(&["new.txt".into()]).await.unwrap();
    assert!(!fx.work.join("new.txt").exists(), "the file goes");
    assert!(state(&git).await.is_empty(), "and nothing is staged");
}

#[tokio::test]
async fn an_action_with_no_paths_touches_nothing() {
    let fx = Fixture::new();
    let git = fx.git();
    std::fs::write(fx.work.join("a.txt"), "three\n").unwrap();
    for empty in [Vec::new(), Vec::new(), Vec::new()] {
        git.stage(&empty).await.unwrap();
        git.discard(&empty).await.unwrap();
    }
    assert_eq!(
        state(&git).await,
        [("a.txt".to_string(), false)],
        "the change is still there, unstaged"
    );
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
    git.stage(&["a.txt".into()]).await.unwrap();

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
