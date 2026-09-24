use groove_types::{
    Annotation, AnnotationId, AnnotationStatus, MrNote, MrThread, NoteOrigin, NotePosition, RepoId,
    SessionId, Timestamp,
};

use crate::merged;

fn own(path: &str, line: u32, at: i64) -> Annotation {
    Annotation {
        id: AnnotationId::new(format!("{path}:{line}")),
        session: SessionId::new("s1"),
        repo: RepoId::new("r1"),
        file_path: path.to_string(),
        start_line: line,
        end_line: line,
        content: "issue: this leaks".into(),
        author: "rsabbah".into(),
        status: AnnotationStatus::Open,
        created_at: Timestamp::new(at),
    }
}

fn note(author: &str, resolved: bool, position: Option<NotePosition>) -> MrNote {
    MrNote {
        author: author.to_string(),
        body: format!("{author} said so"),
        created_at: Timestamp::new(10),
        resolved,
        resolvable: true,
        position,
    }
}

fn on(path: &str, line: u32) -> NotePosition {
    NotePosition {
        new_path: Some(path.to_string()),
        new_line: Some(line),
        ..NotePosition::default()
    }
}

fn thread(id: &str, notes: Vec<MrNote>) -> MrThread {
    MrThread {
        id: id.to_string(),
        notes,
    }
}

#[test]
fn a_note_and_a_thread_read_as_one_list() {
    let threads = vec![thread(
        "t1",
        vec![note("reviewer", false, Some(on("src/lib.rs", 40)))],
    )];
    let read = merged(&[own("src/lib.rs", 10, 1)], &threads);
    assert_eq!(read.len(), 2);
    assert!(read[0].is_local(), "the earlier line first");
    assert_eq!(read[1].origin, NoteOrigin::Thread("t1".into()));
}

#[test]
fn the_list_reads_by_file_then_by_line() {
    let own = [own("b.rs", 1, 1), own("a.rs", 9, 1), own("a.rs", 2, 1)];
    let read = merged(&own, &[]);
    let seen: Vec<(String, u32)> = read
        .iter()
        .filter_map(|one| one.anchor.as_ref())
        .map(|one| (one.path.clone(), one.start_line))
        .collect();
    assert_eq!(
        seen,
        vec![
            ("a.rs".to_string(), 2),
            ("a.rs".to_string(), 9),
            ("b.rs".to_string(), 1)
        ]
    );
}

#[test]
fn a_comment_on_the_merge_request_has_no_line_and_reads_last() {
    let threads = vec![
        thread("general", vec![note("ci-bot", false, None)]),
        thread("t1", vec![note("reviewer", false, Some(on("z.rs", 1)))]),
    ];
    let read = merged(&[], &threads);
    assert_eq!(read.len(), 2);
    assert!(read[0].anchor.is_some(), "an anchored thread reads first");
    assert_eq!(read[1].origin, NoteOrigin::Thread("general".into()));
    assert!(read[1].anchor.is_none());
}

#[test]
fn a_thread_keeps_its_replies_in_order() {
    let threads = vec![thread(
        "t1",
        vec![
            note("reviewer", true, Some(on("a.rs", 3))),
            note("haoov", true, None),
        ],
    )];
    let read = merged(&[], &threads);
    let one = read.first().expect("the thread");
    assert_eq!(one.replies(), 1);
    assert_eq!(
        one.opening().map(|said| said.author.as_str()),
        Some("reviewer")
    );
    assert!(one.resolved, "every note of it is resolved");
}

#[test]
fn a_thread_is_open_while_one_note_of_it_is() {
    let threads = vec![thread(
        "t1",
        vec![
            note("reviewer", true, Some(on("a.rs", 3))),
            note("haoov", false, None),
        ],
    )];
    let read = merged(&[], &threads);
    assert!(!read[0].resolved);
}

#[test]
fn a_thread_that_says_nothing_is_no_note() {
    let read = merged(&[], &[thread("empty", vec![])]);
    assert!(read.is_empty());
}

#[test]
fn a_note_stands_on_every_row_of_its_range() {
    let mut one = own("src/lib.rs", 10, 1);
    one.end_line = 12;
    let read = merged(&[one], &[]);
    let note = read.first().expect("the note");
    assert!(note.on("src/lib.rs", 11));
    assert!(!note.on("src/lib.rs", 13));
    assert!(!note.on("other.rs", 11));
}

#[test]
fn a_posted_note_comes_back_on_the_line_it_left() {
    let local = own("src/lib.rs", 9, 1);
    let up = crate::service::notes::posted(&local);
    assert_eq!((up.from, up.to), (10, 10), "the forge counts from one");

    let threads = vec![thread(
        "t1",
        vec![note("me", false, Some(on("src/lib.rs", up.from)))],
    )];
    let read = merged(&[], &threads);
    let anchor = read[0].anchor.as_ref().expect("on a line");
    assert_eq!(
        (anchor.start_line, anchor.end_line),
        (local.start_line, local.end_line),
        "and comes back where the note stood"
    );
}
