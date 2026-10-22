//! What a new merge request says: its title, its body, and the task it names.

use groove_types::Task;

/// The text an MR is opened or written again with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub title: String,
    pub body: String,
}

/// The task's title, or the branch where there is no task; a footer names the task.
pub fn text_of(task: Option<&Task>, branch: &str) -> Text {
    Text {
        title: task.map_or_else(|| branch.to_string(), |task| task.title.clone()),
        body: task.map(footed).unwrap_or_default(),
    }
}

/// The task's own line, by url where it has one.
fn footed(task: &Task) -> String {
    match task.url.as_deref() {
        Some(url) if !url.is_empty() => format!("Task: {url}"),
        _ => format!("Task: {}", task.external_id),
    }
}
