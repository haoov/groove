//! What a write says to the human who decides on it: its verb and its whole text.

use serde_json::Value;

/// The verb the rail puts after "asks to".
pub fn verb(tool: &str) -> &'static str {
    if tool == crate::APPLY_EDIT {
        return "apply edit to a cluster object";
    }
    crate::named(tool).map_or("write", |one| one.verb)
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
        "spawn_session" => joined(&said("title"), &said("brief")),
        "log_work" => joined(&said("what"), &said("why")),
        "git_push" => joined(&said("branch"), &listed(&arguments["commits"])),
        "kube_exec" => joined(&said("target"), &said("shown")),
        "kube_patch" | "kube_delete" | "argo_sync" | crate::APPLY_EDIT => {
            joined(&said("target"), &said("diff"))
        }
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
