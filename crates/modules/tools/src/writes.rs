//! What the agent may change: git, the forge, the task's own rows, and its notes.

use crate::wording::{SKILL, SUBJECT, TARGET_BRANCH};
use crate::{Tool, takes, task, text, worktree};

mod delivery;

/// A write's name, the verb a human reads it by, and the arguments that name what it acts on.
struct Does {
    name: &'static str,
    verb: &'static str,
    subject: &'static [&'static str],
}

fn does(name: &'static str, verb: &'static str, subject: &'static [&'static str]) -> Does {
    Does {
        name,
        verb,
        subject,
    }
}

/// One tool a human decides on before it runs.
fn write(does: Does, description: &str, schema: serde_json::Value) -> Tool {
    Tool {
        name: does.name,
        description: description.to_string(),
        schema,
        writes: true,
        verb: does.verb,
        subject: does.subject,
    }
}

pub(crate) fn all() -> Vec<Tool> {
    let mut out = git();
    out.extend(delivery::forge());
    out.extend(delivery::notes());
    out.extend(delivery::posting());
    out.extend(delivery::threads());
    out.extend(repos());
    out.extend(skills());
    out.extend(rows());
    out
}

fn git() -> Vec<Tool> {
    vec![
        write(
            does("git_commit", "commit", &["message"]),
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
            does("git_push", "push", &["branch"]),
            "Push the branch to its own name on origin.",
            takes(&["worktree_id"], vec![worktree()]),
        ),
        write(
            does("git_pull", "pull", &[]),
            "Pull from origin, fast-forward only.",
            takes(&["worktree_id"], vec![worktree()]),
        ),
    ]
}

fn repos() -> Vec<Tool> {
    vec![
        write(
            does("add_task_repo", "add a repo", &["repo"]),
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
            does("add_task_worktree", "add a worktree", &["branch"]),
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

/// What the user's own skills take.
fn skills() -> Vec<Tool> {
    vec![write(
        does("save_user_skill", "save a skill", &["name"]),
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
            does("log_task_hours", "log hours", &["task_id"]),
            "Hand the source the hours the clock measured and has not told it about.",
            takes(&[], vec![task()]),
        ),
        write(
            does("finish_task", "finish the task", &["task_id"]),
            "Close the task: every worktree merged or closed, and the source told.",
            takes(&[], vec![task()]),
        ),
        write(
            does("adopt_task", "become a task", &["task"]),
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
