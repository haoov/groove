//! The writes on the merge request and its notes: opened, said on, resolved.

use crate::wording::{NOTE, SUBJECT, mr_description};
use crate::{Tool, number, takes, text, worktree};

use super::{does, write};

pub(super) fn forge() -> Vec<Tool> {
    vec![
        write(
            does("create_mr", "open an MR", &["title"]),
            "Open a merge request from the worktree's branch onto the branch it is based on.",
            takes(
                &["worktree_id", "title"],
                vec![
                    worktree(),
                    ("title", text(SUBJECT)),
                    ("description", text(mr_description())),
                ],
            ),
        ),
        write(
            does("update_mr", "update the MR", &["title"]),
            "Write the merge request's title and body again.",
            takes(
                &["worktree_id"],
                vec![
                    worktree(),
                    ("title", text(SUBJECT)),
                    ("description", text(mr_description())),
                ],
            ),
        ),
        write(
            does("close_mr", "close the MR", &[]),
            "Close the merge request with nothing merged.",
            takes(&["worktree_id"], vec![worktree()]),
        ),
        write(
            does("comment_mr", "comment on the MR", &["body"]),
            "A comment on the merge request itself, under no line.",
            takes(
                &["worktree_id", "body"],
                vec![worktree(), ("body", text("Markdown. Short."))],
            ),
        ),
    ]
}

pub(super) fn notes() -> Vec<Tool> {
    vec![
        write(
            does("create_annotation", "leave a note", &["path"]),
            "Leave a note on a line, or on a range of them. One note a line: a line that \
             already carries an open note of this session takes no other.",
            takes(
                &["path", "line", "content"],
                vec![
                    ("path", text("The file, from the worktree's root.")),
                    (
                        "line",
                        number("The line it stands on, as the file numbers it."),
                    ),
                    (
                        "end_line",
                        number("The last line, for a note over a range."),
                    ),
                    ("content", text(NOTE)),
                    worktree(),
                ],
            ),
        ),
        write(
            does("update_annotation", "edit a note", &["id"]),
            "Write a note's words again. Its lines and its author stay as they are.",
            takes(
                &["id", "content"],
                vec![
                    ("id", text("The note's id, from get_annotations.")),
                    ("content", text(NOTE)),
                ],
            ),
        ),
        write(
            does("resolve_annotation", "resolve a note", &["id"]),
            "Mark a note dealt with. It stays on the list, quiet.",
            takes(
                &["id"],
                vec![("id", text("The note's id, from get_annotations."))],
            ),
        ),
    ]
}

/// What sends a note of this session up to the forge.
pub(super) fn posting() -> Vec<Tool> {
    vec![write(
        does("post_annotation", "post a note", &["id"]),
        "Post a note of this session on the merge request, at its own line. It is \
             resolved here once it is up there.",
        takes(
            &["id"],
            vec![("id", text("The note's id, from get_annotations."))],
        ),
    )]
}

/// What the forge's own discussions take.
pub(super) fn threads() -> Vec<Tool> {
    vec![
        write(
            does("reply_thread", "reply", &["body"]),
            "Answer a thread of the merge request, under what it says.",
            takes(
                &["thread", "body"],
                vec![
                    ("thread", text("The thread's id, from get_mr_threads.")),
                    ("body", text(NOTE)),
                ],
            ),
        ),
        write(
            does("resolve_thread", "resolve a thread", &["thread"]),
            "Resolve a thread of the merge request, or open it again.",
            takes(
                &["thread"],
                vec![
                    ("thread", text("The thread's id, from get_mr_threads.")),
                    (
                        "resolve",
                        serde_json::json!({
                            "type": "boolean",
                            "description": "False opens it again. Defaults to true."
                        }),
                    ),
                ],
            ),
        ),
    ]
}
