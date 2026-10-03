//! What a write says to the human who decides on it: its verb and its whole text.

use serde_json::Value;

/// The verb the rail puts after "asks to".
pub fn verb(tool: &str) -> &'static str {
    match tool {
        "git_commit" => "commit",
        "git_push" => "push",
        "git_pull" => "pull",
        "create_mr" => "open an MR",
        "update_mr" => "update the MR",
        "close_mr" => "close the MR",
        "comment_mr" => "comment on the MR",
        "reply_thread" => "reply",
        "resolve_thread" => "resolve a thread",
        "finish_task" => "finish the task",
        "log_task_hours" => "log hours",
        "add_task_repo" => "add a repo",
        "add_task_worktree" => "add a worktree",
        "save_user_skill" => "save a skill",
        "adopt_task" => "become a task",
        _ => "write",
    }
}

/// Everything the write carries that a human reads before deciding on it.
pub fn said(tool: &str, arguments: &Value) -> String {
    let said = |name: &str| arguments[name].as_str().unwrap_or_default().to_string();
    match tool {
        "git_commit" => said("message"),
        "create_mr" | "update_mr" => joined(&said("title"), &said("description")),
        "comment_mr" | "reply_thread" => said("body"),
        "save_user_skill" => joined(&said("name"), &said("body")),
        "add_task_repo" => joined(&said("repo"), &said("branch")),
        "add_task_worktree" => said("branch"),
        "log_task_hours" => joined("the hours the clock measured", &said("task")),
        "adopt_task" => said("task"),
        "git_push" => joined(&said("branch"), &listed(&arguments["commits"])),
        _ => shown(arguments),
    }
}

/// The worktree a write acts in, when its arguments name one.
pub fn acts_in(arguments: &Value) -> Option<String> {
    arguments["worktree_id"].as_str().map(str::to_string)
}

fn listed(lines: &Value) -> String {
    let lines = lines.as_array().map(Vec::as_slice).unwrap_or_default();
    let lines: Vec<&str> = lines.iter().filter_map(Value::as_str).collect();
    lines.join("\n")
}

fn joined(head: &str, body: &str) -> String {
    match body.is_empty() {
        true => head.to_string(),
        false => format!("{head}\n\n{body}"),
    }
}

/// The arguments as they are, for a write with no text of its own.
fn shown(arguments: &Value) -> String {
    let mut out = String::new();
    if let Some(map) = arguments.as_object() {
        for (name, value) in map.iter().filter(|(name, _)| *name != "worktree_id") {
            let value = value
                .as_str()
                .map(str::to_string)
                .unwrap_or(value.to_string());
            out.push_str(&format!("{name}: {value}\n"));
        }
    }
    out.trim_end().to_string()
}
