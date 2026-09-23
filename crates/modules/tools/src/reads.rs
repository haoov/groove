//! What the agent may ask about: its task, its change, its merge request, its notes.

use crate::{Tool, nothing, number, takes, task, text, worktree};

/// One tool that answers without changing anything.
fn read(name: &'static str, description: &str, schema: serde_json::Value) -> Tool {
    Tool {
        name,
        description: description.to_string(),
        schema,
        writes: false,
    }
}

pub(crate) fn all() -> Vec<Tool> {
    let mut out = about();
    out.extend(work());
    out.extend(skills());
    out.extend(files());
    out
}

/// What the session can be sent, and what one of those skills says.
fn skills() -> Vec<Tool> {
    vec![
        read(
            "list_skills",
            "Every skill this session can be sent, core and the user's own, with what each \
             one is for.",
            nothing(),
        ),
        read(
            "read_user_skill",
            "One skill of the user's own, as its file stands. Read it before you write it \
             again, so nothing of theirs is lost.",
            takes(
                &["name"],
                vec![("name", text("The skill's own name, from list_skills."))],
            ),
        ),
    ]
}

/// What the session itself is.
fn about() -> Vec<Tool> {
    vec![
        read(
            "get_active_task",
            "Get the currently open task, its worktrees, and repos.",
            nothing(),
        ),
        read(
            "list_tasks",
            "Every real task the app knows about, with status and priority. Use it to \
             answer what is queued or in progress.",
            nothing(),
        ),
        read(
            "list_repos",
            "Every repo cloned locally, each flagged `attached` when it is already on your \
             task, and each with the `local_path` of its clone. Use it for the exact name \
             add_task_repo wants, or for the path to read a repo this session has not \
             checked out.",
            nothing(),
        ),
    ]
}

/// What the work has come to.
fn work() -> Vec<Tool> {
    vec![
        read(
            "get_task_diff",
            "Diff for all worktrees of a task, against the branch each one is based on.",
            takes(&[], vec![task()]),
        ),
        read(
            "get_commit_log",
            "Commit history for all worktrees of a task, the branch's own first.",
            takes(
                &[],
                vec![task(), ("limit", number("How many commits at most."))],
            ),
        ),
        read(
            "get_mr_state",
            "MR/PR state for a worktree.",
            takes(&["worktree_id"], vec![worktree()]),
        ),
        read(
            "get_mr_threads",
            "The discussion on an MR, live from the forge: review threads with their file \
             and line, and general comments such as a CI bot's report. Each note says who \
             wrote it and whether its thread is resolved. A reviewer's thread is a note to \
             act on, the same as an annotation.",
            takes(&["worktree_id"], vec![worktree()]),
        ),
        read(
            "get_mr_ci",
            "Pipeline status and the run's URL for an MR, live from the forge. Use the \
             forge CLI (glab ci / gh run) to read the failing job's log — this returns the \
             status, not the log.",
            takes(&["worktree_id"], vec![worktree()]),
        ),
    ]
}

/// What stands on the files themselves.
fn files() -> Vec<Tool> {
    vec![
        read(
            "get_annotations",
            "Every note on this session: the ones left here, and the merge request's own \
             threads, each with its file and line.",
            takes(&[], vec![task()]),
        ),
        read(
            "get_open_file",
            "The file the user has open, and where their caret stands in it.",
            nothing(),
        ),
        read(
            "get_status",
            "What each worktree of the task has: its branch, what is staged, what is \
             changed, and how far it stands from origin.",
            takes(&[], vec![task()]),
        ),
        read(
            "read_file",
            "One file of a worktree as it stands on disk, or as one commit left it.",
            takes(
                &["worktree_id", "path"],
                vec![
                    worktree(),
                    ("path", text("The file, from the worktree's root.")),
                    (
                        "commit",
                        text("Read it as this commit left it, not the disk."),
                    ),
                ],
            ),
        ),
    ]
}
