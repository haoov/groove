//! The merge request's threads, as the notes the surface lists.

use groove_types::{Anchor, MrNote, MrThread, Note, Said};

/// The threads of the merge request, in the order a file reads.
pub fn listed(threads: &[MrThread]) -> Vec<Note> {
    let mut out: Vec<Note> = threads.iter().filter_map(of_thread).collect();
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

/// A thread the forge answered; one with nothing said, or that nobody can resolve, is none.
fn of_thread(thread: &MrThread) -> Option<Note> {
    let first = thread.notes.first().filter(|one| one.resolvable)?;
    Some(Note {
        thread: thread.id.clone(),
        anchor: anchor_of(first),
        resolved: thread.notes.iter().all(|note| note.resolved),
        said: thread.notes.iter().map(said_of).collect(),
    })
}

/// Where the thread hangs on the new side; a line 0 is none.
fn anchor_of(note: &MrNote) -> Option<Anchor> {
    let position = note.position.as_ref()?;
    let path = position.new_path.clone()?;
    let line = |one: u32| (one > 0).then_some(i64::from(one));
    let start = position.new_line.and_then(line)?;
    let end = position.end_new_line.and_then(line);
    Anchor::numbered(path, start, end)
}

fn said_of(note: &MrNote) -> Said {
    Said {
        author: note.author.clone(),
        body: note.body.clone(),
        at: note.created_at,
    }
}
