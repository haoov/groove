use crate::tests::fixture::{Fixture, sh};

#[tokio::test]
async fn the_log_reads_the_newest_commits_first() {
    let fx = Fixture::new();
    let git = fx.git();
    let read = git.log(None, 10).await.unwrap();
    let said: Vec<&str> = read.iter().map(|one| one.message.as_str()).collect();
    assert_eq!(said, ["second", "first"]);
    let one = read.first().expect("a commit");
    assert_eq!(one.author, "T");
    assert_eq!(one.short_sha.len(), 7, "{one:?}");
    assert!(one.sha.starts_with(&one.short_sha));
    assert!(one.at.seconds() > 0, "when it was made");
}

#[tokio::test]
async fn the_log_stops_at_the_limit() {
    let fx = Fixture::new();
    let read = fx.git().log(None, 1).await.unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].message, "second");
}

#[tokio::test]
async fn the_commits_the_base_already_had_are_its_own() {
    let fx = Fixture::new();
    let read = fx.git().log(Some("origin/main"), 10).await.unwrap();
    let base: Vec<bool> = read.iter().map(|one| one.is_base).collect();
    assert_eq!(base, [false, true], "the unpushed one is the branch's");
}

#[tokio::test]
async fn a_commit_says_which_files_it_changed() {
    let fx = Fixture::new();
    let git = fx.git();
    std::fs::write(fx.work.join("b.txt"), "b\n").unwrap();
    sh(&fx.work, &["add", "."]);
    sh(&fx.work, &["commit", "-m", "third"]);
    let sha = sh(&fx.work, &["rev-parse", "HEAD"]);

    let read = git.changed_in(&sha).await.unwrap();
    let paths: Vec<&str> = read.iter().map(|one| one.path.as_str()).collect();
    assert_eq!(paths, ["b.txt"]);
    assert_eq!(read[0].added, Some(1));
    assert_eq!(read[0].deleted, Some(0));
}

#[tokio::test]
async fn a_commit_gives_both_sides_of_what_it_touched() {
    let fx = Fixture::new();
    let git = fx.git();
    let sha = sh(&fx.work, &["rev-parse", "HEAD"]);
    let paths = vec!["a.txt".to_string()];

    let (before, after) = git.sides_in(&sha, &paths).await.unwrap();
    assert_eq!(before.get("a.txt").map(String::as_str), Some("one\n"));
    assert_eq!(after.get("a.txt").map(String::as_str), Some("two\n"));
}

#[tokio::test]
async fn the_first_commit_of_all_has_nothing_before_it() {
    let fx = Fixture::new();
    let git = fx.git();
    let sha = sh(&fx.work, &["rev-list", "--max-parents=0", "HEAD"]);
    let paths = vec!["a.txt".to_string()];

    let (before, after) = git.sides_in(&sha, &paths).await.unwrap();
    assert!(before.is_empty(), "no parent to read");
    assert_eq!(after.get("a.txt").map(String::as_str), Some("one\n"));
}

#[tokio::test]
async fn a_message_with_a_nul_free_subject_survives_the_read() {
    let fx = Fixture::new();
    let git = fx.git();
    std::fs::write(fx.work.join("a.txt"), "three\n").unwrap();
    sh(
        &fx.work,
        &["commit", "-am", "fix(ui): keep\ttabs and · dots"],
    );
    let read = git.log(None, 1).await.unwrap();
    assert_eq!(read[0].message, "fix(ui): keep\ttabs and · dots");
}
