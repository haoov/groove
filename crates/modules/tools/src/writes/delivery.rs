//! The writes on the merge request and its threads: opened, said on, resolved.

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
    ]
}

/// A comment on the MR, on lines of the diff or on the MR itself.
pub(super) fn comment() -> Tool {
    write(
        does("comment_mr", "comment on the MR", &["body"]),
        "A comment on the merge request. With a path and a line it opens a thread on \
         those lines of the diff; without, it stands on the MR itself.",
        takes(
            &["worktree_id", "body"],
            vec![
                worktree(),
                ("body", text(NOTE)),
                ("path", text("The file, from the worktree's root.")),
                (
                    "line",
                    number("The line it stands on, as the file numbers it."),
                ),
                ("end_line", number("The last line, for a range.")),
            ],
        ),
    )
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
