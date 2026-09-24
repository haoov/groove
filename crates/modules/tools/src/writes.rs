//! What the agent may change: git, the forge, the task's own rows, and its notes.

use crate::wording::{NOTE, SKILL, SUBJECT, TARGET_BRANCH, mr_description};
use crate::{Tool, number, takes, task, text, worktree};

/// One tool a human decides on before it runs.
fn write(name: &'static str, description: &str, schema: serde_json::Value) -> Tool {
    Tool {
        name,
        description: description.to_string(),
        schema,
        writes: true,
    }
}

pub(crate) fn all() -> Vec<Tool> {
    let mut out = git();
    out.extend(forge());
    out.extend(notes());
    out.extend(posting());
    out.extend(threads());
    out.extend(repos());
    out.extend(skills());
    out.extend(rows());
    out
}

fn git() -> Vec<Tool> {
    vec![
        write(
            "git_commit",
            "Commit what is STAGED, and only that. Stage first and stage deliberately: \
             `git add <paths>` for some files, `git add -A` for everything including new \
             ones. Nothing staged is an error, never a commit of everything.",
            takes(
                &["worktree_id", "message"],
                vec![
                    worktree(),
                    (
                        "message",
                        text(format!(
                            "{SUBJECT} Body optional: why, not what. No file lists."
                        )),
                    ),
                ],
            ),
        ),
        write(
            "git_push",
            "Push the branch to its own name on origin.",
            takes(&["worktree_id"], vec![worktree()]),
        ),
        write(
            "git_pull",
            "Pull from origin, fast-forward only.",
            takes(&["worktree_id"], vec![worktree()]),
        ),
    ]
}

fn forge() -> Vec<Tool> {
    vec![
        write(
            "create_mr",
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
            "update_mr",
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
            "close_mr",
            "Close the merge request with nothing merged.",
            takes(&["worktree_id"], vec![worktree()]),
        ),
        write(
            "comment_mr",
            "A comment on the merge request itself, under no line.",
            takes(
                &["worktree_id", "body"],
                vec![worktree(), ("body", text("Markdown. Short."))],
            ),
        ),
    ]
}

fn notes() -> Vec<Tool> {
    vec![
        write(
            "create_annotation",
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
            "update_annotation",
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
            "resolve_annotation",
            "Mark a note dealt with. It stays on the list, quiet.",
            takes(
                &["id"],
                vec![("id", text("The note's id, from get_annotations."))],
            ),
        ),
    ]
}

/// What sends a note of this session up to the forge.
fn posting() -> Vec<Tool> {
    vec![write(
        "post_annotation",
        "Post a note of this session on the merge request, at its own line. It is \
             resolved here once it is up there.",
        takes(
            &["id"],
            vec![("id", text("The note's id, from get_annotations."))],
        ),
    )]
}

/// What the forge's own discussions take.
fn threads() -> Vec<Tool> {
    vec![
        write(
            "reply_thread",
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
            "resolve_thread",
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

fn repos() -> Vec<Tool> {
    vec![
        write(
            "add_task_repo",
            "Attach a repo to your task and make its worktree. The repo must already be \
             cloned locally — call list_repos first; this tool cannot clone.",
            takes(
                &["repo"],
                vec![
                    (
                        "repo",
                        text("Slug (group/path/project) or project name from list_repos."),
                    ),
                    task(),
                    (
                        "branch",
                        text("Branch for the new worktree. Defaults to the task branch."),
                    ),
                    ("target_branch", text(TARGET_BRANCH)),
                ],
            ),
        ),
        write(
            "add_task_worktree",
            "Check out ANOTHER branch of a repo your task already has, as a second worktree \
             beside the first. Use add_task_repo for a repo the task does not have yet.",
            takes(
                &["branch"],
                vec![
                    (
                        "branch",
                        text(
                            "Must differ from the ones already checked out; include the task \
                              id so it stays traceable.",
                        ),
                    ),
                    (
                        "repo",
                        text(
                            "Project name from get_active_task. Only needed where the \
                                   task has more than one repo.",
                        ),
                    ),
                    task(),
                    ("target_branch", text(TARGET_BRANCH)),
                ],
            ),
        ),
    ]
}

/// What the task's own rows take.
/// What the user's own skills take.
fn skills() -> Vec<Tool> {
    vec![write(
        "save_user_skill",
        "Write one skill of the user's own. Read it first with read_user_skill and keep \
         what they wrote. The body is a SKILL.md: front matter, then what to do.",
        takes(
            &["name", "body"],
            vec![
                (
                    "name",
                    text("Lower case letters, digits and dashes; it becomes a directory."),
                ),
                ("body", text(SKILL)),
                (
                    "previous",
                    text("The name this one replaces, when it is a rename or a rewrite."),
                ),
            ],
        ),
    )]
}

fn rows() -> Vec<Tool> {
    vec![
        write(
            "log_task_hours",
            "Hand the source the hours the clock measured and has not told it about.",
            takes(&[], vec![task()]),
        ),
        write(
            "finish_task",
            "Close the task: every worktree merged or closed, and the source told.",
            takes(&[], vec![task()]),
        ),
        write(
            "adopt_task",
            "Make this explorer the session of a task you have filed at its source: a \
             Notion page or a GitHub issue you created. The session keeps its repos and \
             its worktrees. Groove never files the task itself.",
            takes(
                &["task"],
                vec![(
                    "task",
                    text("The task's URL, as its source shows it, or its id."),
                )],
            ),
        ),
    ]
}
