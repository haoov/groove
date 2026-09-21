//! The session's own notes and the merge request's threads, as one list.

use groove_types::{
    Anchor, Annotation, AnnotationStatus, MrNote, MrThread, Note, NoteOrigin, Said,
};

/// The notes of a session, then its threads, in the order a file reads.
pub fn merged(own: &[Annotation], threads: &[MrThread]) -> Vec<Note> {
    let mut out: Vec<Note> = own.iter().map(of_annotation).collect();
    out.extend(threads.iter().filter_map(of_thread));
    out.sort_by_key(by_place);
    out
}

/// What a note sorts by: its file, its first line, then when it was opened.
fn by_place(note: &Note) -> (bool, String, u32, i64) {
    let anchor = note.anchor.as_ref();
    (
        anchor.is_none(),
        anchor.map(|one| one.path.clone()).unwrap_or_default(),
        anchor.map_or(0, |one| one.start_line),
        note.at().map_or(0, |at| at.seconds()),
    )
}

fn of_annotation(one: &Annotation) -> Note {
    Note {
        origin: NoteOrigin::Local(one.id.clone()),
        anchor: Some(Anchor {
            path: one.file_path.clone(),
            start_line: one.start_line,
            end_line: one.end_line,
        }),
        resolved: one.status == AnnotationStatus::Resolved,
        said: vec![Said {
            author: one.author.clone(),
            body: one.content.clone(),
            at: one.created_at,
        }],
    }
}

/// A thread the forge answered. One with nothing said is not a note.
fn of_thread(thread: &MrThread) -> Option<Note> {
    let first = thread.notes.first()?;
    Some(Note {
        origin: NoteOrigin::Thread(thread.id.clone()),
        anchor: anchor_of(first),
        resolved: thread.notes.iter().all(|note| note.resolved),
        said: thread.notes.iter().map(said_of).collect(),
    })
}

/// Where the thread hangs, when the forge put it on a line of the new side.
fn anchor_of(note: &MrNote) -> Option<Anchor> {
    let position = note.position.as_ref()?;
    let path = position.new_path.clone()?;
    let start = position.new_line?;
    Some(Anchor {
        path,
        start_line: start,
        end_line: position.end_new_line.unwrap_or(start).max(start),
    })
}

fn said_of(note: &MrNote) -> Said {
    Said {
        author: note.author.clone(),
        body: note.body.clone(),
        at: note.created_at,
    }
}
