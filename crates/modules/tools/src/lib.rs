//! What the agent may ask of Groove: one definition per tool, as the harness lists them.

pub mod answers;
mod arguments;
mod reads;
mod said;
pub mod wording;
mod writes;

pub use arguments::Arguments;
pub use said::{acts_in, said, verb};

#[cfg(test)]
mod tests;

/// One tool, as `tools/list` answers for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: &'static str,
    pub description: String,
    /// The arguments it takes, as JSON Schema.
    pub schema: serde_json::Value,
    /// It changes something, and a human decides first.
    pub writes: bool,
    /// What the rail says it asks to do, after "asks to".
    pub verb: &'static str,
    /// The arguments that name what it acts on, the first one given wins.
    pub subject: &'static [&'static str],
}

impl Tool {
    /// What `tools/list` sends back for it.
    pub fn listed(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.schema,
        })
    }
}

/// Every tool Groove answers, reads first.
pub fn all() -> Vec<Tool> {
    let mut out = reads::all();
    out.extend(writes::all());
    out
}

/// The tool one name stands for.
pub fn named(name: &str) -> Option<Tool> {
    all().into_iter().find(|tool| tool.name == name)
}

/// A tool that only reads takes no object at all.
pub(crate) fn nothing() -> serde_json::Value {
    serde_json::json!({ "type": "object", "properties": {} })
}

/// The schema of a tool, from its required names and every property it takes.
pub(crate) fn takes(
    required: &[&str],
    properties: Vec<(&str, serde_json::Value)>,
) -> serde_json::Value {
    let properties: serde_json::Map<String, serde_json::Value> = properties
        .into_iter()
        .map(|(name, one)| (name.to_string(), one))
        .collect();
    serde_json::json!({
        "type": "object",
        "required": required,
        "properties": properties,
    })
}

/// One string argument, with what it is for.
pub(crate) fn text(description: impl Into<String>) -> serde_json::Value {
    serde_json::json!({ "type": "string", "description": description.into() })
}

/// One whole-number argument.
pub(crate) fn number(description: impl Into<String>) -> serde_json::Value {
    serde_json::json!({ "type": "integer", "description": description.into() })
}

/// The worktree a write acts on, which every one of them names.
pub(crate) fn worktree() -> (&'static str, serde_json::Value) {
    (
        "worktree_id",
        text("The worktree to act on, from get_active_task."),
    )
}

/// The task a read is about, your own by default.
pub(crate) fn task() -> (&'static str, serde_json::Value) {
    ("task_id", text("Defaults to your own task."))
}

/// What a write acts on, as a human reads it on the row that asks.
pub fn subject(tool: &str, arguments: &serde_json::Value) -> String {
    let named: &[&str] = named(tool).map_or(&[], |one| one.subject);
    let fallback = ["thread", "id", "worktree_id", "task_id"];
    let said = named
        .iter()
        .chain(fallback.iter())
        .find_map(|name| arguments[*name].as_str());
    said.and_then(|one| one.lines().next())
        .unwrap_or_default()
        .to_string()
}
