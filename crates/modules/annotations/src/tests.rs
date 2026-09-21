use groove_types::{AnnotationId, AnnotationStatus, RepoId, SessionId, Timestamp};

use crate::{New, Store};

/// A store whose session and repo rows the notes can point at.
async fn store() -> Store {
    let store = Store::in_memory().await.expect("a store");
    sqlx::query(
        "INSERT INTO sessions (id, kind, title, created_at) VALUES ('s', 'explorer', 'A', 0)",
    )
    .execute(store.db().pool())
    .await
    .expect("a session");
    sqlx::query(
        "INSERT INTO repos (id, host, group_path, project, local_path)
         VALUES ('h/g/p', 'h', 'g', 'p', '/pool/p')",
    )
    .execute(store.db().pool())
    .await
    .expect("a repo");
    store
}

fn note(path: &str, start: u32, end: u32) -> New {
    New {
        session: SessionId::new("s"),
        repo: RepoId::new("h/g/p"),
        file_path: path.to_string(),
        start_line: start,
        end_line: end,
        content: "issue: this leaks".to_string(),
        author: "rsabbah".to_string(),
    }
}

fn at(seconds: i64) -> Timestamp {
    Timestamp::new(seconds)
}

#[tokio::test]
async fn a_note_is_written_open_and_read_back() {
    let store = store().await;
    let made = store
        .create(note("src/lib.rs", 10, 12), at(1))
        .await
        .expect("a note");
    assert_eq!(made.status, AnnotationStatus::Open);
    assert_eq!((made.start_line, made.end_line), (10, 12));
    let read = store.list(&SessionId::new("s")).await.expect("the notes");
    assert_eq!(read, vec![made]);
}

#[tokio::test]
async fn a_selection_made_upwards_keeps_the_lower_line_first() {
    let store = store().await;
    let made = store
        .create(note("src/lib.rs", 12, 10), at(1))
        .await
        .expect("a note");
    assert_eq!((made.start_line, made.end_line), (10, 12));
}

#[tokio::test]
async fn the_notes_of_a_session_read_in_the_order_a_file_reads() {
    let store = store().await;
    for (path, line) in [("b.rs", 4), ("a.rs", 9), ("a.rs", 2)] {
        store
            .create(note(path, line, line), at(1))
            .await
            .expect("a note");
    }
    let read = store.list(&SessionId::new("s")).await.expect("the notes");
    let seen: Vec<(String, u32)> = read
        .into_iter()
        .map(|one| (one.file_path, one.start_line))
        .collect();
    assert_eq!(
        seen,
        vec![
            ("a.rs".to_string(), 2),
            ("a.rs".to_string(), 9),
            ("b.rs".to_string(), 4),
        ]
    );
}

#[tokio::test]
async fn another_session_reads_none_of_them() {
    let store = store().await;
    store
        .create(note("src/lib.rs", 1, 1), at(1))
        .await
        .expect("a note");
    let read = store
        .list(&SessionId::new("other"))
        .await
        .expect("the notes");
    assert!(read.is_empty());
}

#[tokio::test]
async fn new_words_replace_the_old_ones() {
    let store = store().await;
    let made = store
        .create(note("src/lib.rs", 1, 1), at(1))
        .await
        .expect("a note");
    let said = store
        .update(&made.id, "nitpick: name it")
        .await
        .expect("the note");
    assert_eq!(said.content, "nitpick: name it");
    assert_eq!(said.created_at, made.created_at, "the time it was made");
}

#[tokio::test]
async fn a_note_resolves_and_opens_again() {
    let store = store().await;
    let made = store
        .create(note("src/lib.rs", 1, 1), at(1))
        .await
        .expect("a note");
    let shut = store.resolve(&made.id).await.expect("resolved");
    assert_eq!(shut.status, AnnotationStatus::Resolved);
    let again = store.reopen(&made.id).await.expect("open");
    assert_eq!(again.status, AnnotationStatus::Open);
}

#[tokio::test]
async fn a_deleted_note_is_gone_from_the_session() {
    let store = store().await;
    let made = store
        .create(note("src/lib.rs", 1, 1), at(1))
        .await
        .expect("a note");
    store.delete(&made.id).await.expect("deleted");
    let read = store.list(&SessionId::new("s")).await.expect("the notes");
    assert!(read.is_empty());
    assert!(store.get(&made.id).await.expect("a read").is_none());
}

#[tokio::test]
async fn a_write_to_a_note_that_is_gone_says_so() {
    let store = store().await;
    let id = AnnotationId::new("nothing");
    let refused = store.resolve(&id).await.expect_err("no such note");
    assert!(refused.message.contains("no note nothing"), "{refused:?}");
}
