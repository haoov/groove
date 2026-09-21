//! What a new merge request says: its title, its body, and the task it names.

use groove_types::Task;

/// The text an MR is opened or written again with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub title: String,
    pub body: String,
}

/// The box's first line titles it, the rest is its body, and a footer names the task.
pub fn text_of(message: &str, task: Option<&Task>, branch: &str) -> Text {
    let mut lines = message.trim().lines();
    let first = lines.next().unwrap_or_default().trim();
    let rest = lines.collect::<Vec<&str>>().join("\n").trim().to_string();
    Text {
        title: titled(first, task, branch),
        body: bodied(&rest, task),
    }
}

fn titled(first: &str, task: Option<&Task>, branch: &str) -> String {
    if !first.is_empty() {
        return first.to_string();
    }
    match task {
        Some(task) => task.title.clone(),
        None => branch.to_string(),
    }
}

/// The message's own lines, then the footer that links the task.
fn bodied(rest: &str, task: Option<&Task>) -> String {
    let Some(footer) = task.map(footed) else {
        return rest.to_string();
    };
    match rest.is_empty() {
        true => footer,
        false => format!("{rest}\n\n{footer}"),
    }
}

/// The task's own line, by url where it has one.
fn footed(task: &Task) -> String {
    match task.url.as_deref() {
        Some(url) if !url.is_empty() => format!("Task: {url}"),
        _ => format!("Task: {}", task.external_id),
    }
}
