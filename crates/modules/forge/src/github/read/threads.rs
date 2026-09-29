//! The discussion: the threads anchored in the diff, then the plain comments.

use groove_types::{MrNote, MrThread, NotePosition};

use super::reviews::nodes;
use super::{at, text};

/// Every thread the MR carries. A thread with no note is not one.
pub(super) fn all(pr: &serde_json::Value) -> Vec<MrThread> {
    let anchored = nodes(&pr["reviewThreads"])
        .into_iter()
        .map(|thread| anchored(&thread));
    let loose = nodes(&pr["comments"])
        .into_iter()
        .filter_map(|comment| loose(&comment));
    anchored
        .chain(loose)
        .filter(|thread| !thread.notes.is_empty())
        .collect()
}

/// A review thread: every note in it shares the thread's file and side.
fn anchored(thread: &serde_json::Value) -> MrThread {
    let resolved = thread["isResolved"].as_bool().unwrap_or_default();
    let path = text(&thread["path"]);
    let on_new_side = thread["diffSide"].as_str().unwrap_or("RIGHT") == "RIGHT";
    let start = thread["startLine"]
        .as_u64()
        .or_else(|| thread["originalStartLine"].as_u64())
        .and_then(|one| u32::try_from(one).ok());
    MrThread {
        id: text(&thread["id"]),
        notes: nodes(&thread["comments"])
            .iter()
            .map(|note| MrNote {
                author: text(&note["author"]["login"]),
                body: text(&note["body"]),
                created_at: at(&note["createdAt"]).unwrap_or_default(),
                resolved,
                resolvable: true,
                position: on_new_side
                    .then(|| line(note))
                    .flatten()
                    .map(|line| NotePosition {
                        new_path: Some(path.clone()),
                        new_line: Some(start.unwrap_or(line).min(line)),
                        end_new_line: start.map(|_| line),
                        ..NotePosition::default()
                    }),
            })
            .collect(),
    }
}

/// A comment on the MR itself: one note, anchored nowhere, resolvable never.
fn loose(comment: &serde_json::Value) -> Option<MrThread> {
    let body = text(&comment["body"]);
    if body.is_empty() {
        return None;
    }
    Some(MrThread {
        id: text(&comment["id"]),
        notes: vec![MrNote {
            author: text(&comment["author"]["login"]),
            body,
            created_at: at(&comment["createdAt"]).unwrap_or_default(),
            resolved: false,
            resolvable: false,
            position: None,
        }],
    })
}

/// The line a note stands on now, or the one it was written against.
fn line(note: &serde_json::Value) -> Option<u32> {
    let line = note["line"]
        .as_u64()
        .or_else(|| note["originalLine"].as_u64())?;
    u32::try_from(line).ok()
}
