use groove_types::{SessionId, TimelineEvent, TimelineKind, Timestamp};

use crate::Timeline;

/// A timeline whose two sessions the lines can point at.
async fn timeline() -> Timeline {
    let held = Timeline::in_memory().await.expect("a timeline");
    for id in ["s1", "s2"] {
        sqlx::query(
            "INSERT INTO sessions (id, kind, title, created_at) VALUES (?, 'explorer', 'A', 0)",
        )
        .bind(id)
        .execute(held.db().pool())
        .await
        .expect("a session");
    }
    held
}

fn event(session: &str, at: i64, kind: TimelineKind, subject: &str) -> TimelineEvent {
    TimelineEvent {
        session: SessionId::new(session),
        at: Timestamp::new(at),
        kind,
        subject: subject.to_string(),
        payload: serde_json::json!({ "path": "src/lib.rs" }),
    }
}

async fn said(held: &Timeline, session: &str) -> Vec<String> {
    held.list(&SessionId::new(session), 50)
        .await
        .expect("the lines")
        .into_iter()
        .map(|one| one.subject)
        .collect()
}

#[tokio::test]
async fn a_line_reads_back_as_it_was_written() {
    let held = timeline().await;
    let one = event("s1", 10, TimelineKind::Commit, "fix: one");
    held.append(&one).await.expect("written");
    let read = held.list(&SessionId::new("s1"), 10).await.expect("the log");
    assert_eq!(read, vec![one], "every field, the payload included");
}

#[tokio::test]
async fn the_newest_line_reads_first() {
    let held = timeline().await;
    for (at, subject) in [(10, "first"), (30, "third"), (20, "second")] {
        let one = event("s1", at, TimelineKind::Commit, subject);
        held.append(&one).await.expect("written");
    }
    assert_eq!(said(&held, "s1").await, ["third", "second", "first"]);
}

#[tokio::test]
async fn two_lines_at_the_same_time_read_in_the_order_they_came() {
    let held = timeline().await;
    for subject in ["first", "second"] {
        let one = event("s1", 10, TimelineKind::Commit, subject);
        held.append(&one).await.expect("written");
    }
    assert_eq!(said(&held, "s1").await, ["second", "first"]);
}

#[tokio::test]
async fn the_limit_holds() {
    let held = timeline().await;
    for at in 0..5 {
        let one = event("s1", at, TimelineKind::Commit, &format!("line {at}"));
        held.append(&one).await.expect("written");
    }
    let read = held.list(&SessionId::new("s1"), 2).await.expect("the log");
    assert_eq!(read.len(), 2, "the newest two");
    assert_eq!(read[0].subject, "line 4");
}

#[tokio::test]
async fn another_sessions_lines_are_not_read() {
    let held = timeline().await;
    held.append(&event("s1", 10, TimelineKind::Commit, "mine"))
        .await
        .expect("written");
    held.append(&event("s2", 10, TimelineKind::Commit, "theirs"))
        .await
        .expect("written");
    assert_eq!(said(&held, "s1").await, ["mine"]);
    assert_eq!(said(&held, "s2").await, ["theirs"]);
}

#[tokio::test]
async fn every_kind_survives_the_round_trip() {
    let held = timeline().await;
    for (at, kind) in TimelineKind::ALL.iter().enumerate() {
        let one = event("s1", at as i64, *kind, "x");
        held.append(&one).await.expect("written");
    }
    let mut read: Vec<TimelineKind> = held
        .list(&SessionId::new("s1"), 50)
        .await
        .expect("the log")
        .into_iter()
        .map(|one| one.kind)
        .collect();
    read.reverse();
    assert_eq!(
        read,
        TimelineKind::ALL,
        "each word reads back as its own kind"
    );
}

#[tokio::test]
async fn a_line_of_a_kind_this_version_never_wrote_is_left_out() {
    let held = timeline().await;
    held.append(&event("s1", 10, TimelineKind::Commit, "mine"))
        .await
        .expect("written");
    sqlx::query(
        "INSERT INTO timeline (session_id, at, kind, subject, payload)
         VALUES ('s1', 20, 'from_a_later_groove', 'theirs', '{}')",
    )
    .execute(held.db().pool())
    .await
    .expect("a row from elsewhere");
    assert_eq!(
        said(&held, "s1").await,
        ["mine"],
        "a word nothing here knows is skipped, not guessed at"
    );
}

#[tokio::test]
async fn a_session_thrown_away_takes_its_lines_with_it() {
    let held = timeline().await;
    held.append(&event("s1", 10, TimelineKind::Commit, "mine"))
        .await
        .expect("written");
    sqlx::query("DELETE FROM sessions WHERE id = 's1'")
        .execute(held.db().pool())
        .await
        .expect("the session goes");
    assert!(said(&held, "s1").await.is_empty(), "the rows go with it");
}

#[tokio::test]
async fn a_log_forgotten_leaves_the_other_sessions_alone() {
    let held = timeline().await;
    held.append(&event("s1", 10, TimelineKind::Commit, "mine"))
        .await
        .expect("written");
    held.append(&event("s2", 10, TimelineKind::Commit, "theirs"))
        .await
        .expect("written");
    held.forget(&SessionId::new("s1")).await.expect("forgotten");
    assert!(said(&held, "s1").await.is_empty());
    assert_eq!(said(&held, "s2").await, ["theirs"]);
}
