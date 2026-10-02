use crate::parse::blame;
use crate::tests::fixture::{Fixture, sh};

/// Two lines of one commit, its facts printed once, then a line not committed.
const PORCELAIN: &str = "\
1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b 1 1 2
author Ada Lovelace
author-mail <ada@example.com>
author-time 1700000000
author-tz +0100
summary feat: add the thing
filename a.rs
\tfirst line
1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b 2 2
\tsecond line
0000000000000000000000000000000000000000 3 3 1
author Not Committed Yet
author-time 1700000100
summary Version of a.rs from a.rs
filename a.rs
\tthird line
";

#[test]
fn each_line_says_who_changed_it_and_when() {
    let read = blame(PORCELAIN);
    let said: Vec<(u32, &str, &str, bool)> = read
        .iter()
        .map(|one| {
            (
                one.line,
                one.author.as_str(),
                one.short_sha.as_str(),
                one.uncommitted,
            )
        })
        .collect();
    assert_eq!(
        said,
        [
            (0, "Ada Lovelace", "1a2b3c4", false),
            (1, "Ada Lovelace", "1a2b3c4", false),
            (2, "Not Committed Yet", "0000000", true),
        ],
        "a commit's later lines reuse the facts printed on its first"
    );
    assert_eq!(read[1].at.seconds(), 1_700_000_000);
    assert_eq!(read[1].summary, "feat: add the thing");
}

#[test]
fn a_field_that_reads_like_a_header_is_not_one() {
    let text = "\
1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b 1 1 1
author Ada Lovelace
author-time 1700000000
previous 0b9a8f7e6d5c4b3a2e1d0c9b8a7f6e5d4c3b2a1b dir/a b/c.rs
filename dir/with a space/file.rs
\tonly line
";
    let read = blame(text);
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].sha, "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b");
}

#[tokio::test]
async fn the_buffer_is_blamed_with_its_own_lines_not_committed() {
    let fx = Fixture::new();
    let head = sh(&fx.work, &["rev-parse", "HEAD"]);
    let read = fx
        .git()
        .blame("a.txt", Some("added\ntwo\n".into()))
        .await
        .unwrap();
    let said: Vec<(bool, &str)> = read
        .iter()
        .map(|one| (one.uncommitted, one.sha.as_str()))
        .collect();
    assert_eq!(
        said,
        [
            (true, "0000000000000000000000000000000000000000"),
            (false, head.as_str())
        ]
    );
    assert_eq!(read[1].author, "T");
}

#[tokio::test]
async fn a_file_git_never_held_has_no_blame() {
    let fx = Fixture::new();
    std::fs::write(fx.work.join("new.txt"), "fresh\n").unwrap();
    let read = fx.git().blame("new.txt", None).await.unwrap();
    assert!(read.is_empty());
}
