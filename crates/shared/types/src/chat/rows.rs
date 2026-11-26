//! The rows a conversation draws, and the calls they stand for.

use crate::Timestamp;

/// One row of the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    You(String),
    Said(String),
    /// Thinking, looks, commands and MCP calls back to back, folded into one row.
    Glance(Glance),
    Edit(Edit),
    Tool(Tool),
    /// An agent the agent sent, its steps and its answer.
    Subagent(Subagent),
    Interrupted,
    /// What went wrong: an API error, a reached limit, or the agent ending by itself.
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub id: String,
    pub path: String,
    pub created: bool,
    pub patch: Option<Patch>,
    pub failed: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Glance {
    pub steps: Vec<Step>,
    /// The last thing glanced was a command or a call.
    pub running: bool,
    /// What the last thing glanced does, in a few words.
    pub doing: Option<String>,
    /// From the first thing glanced to the record after the last.
    pub since: Option<Timestamp>,
    pub until: Option<Timestamp>,
}

/// One step of a glance: what it was, what it acted on, and when it began.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub kind: Kind,
    /// The file read, the pattern searched, the command run, the tool called.
    pub target: String,
    /// The command or the call, for a step that runs one.
    pub run: Option<Run>,
    pub at: Option<Timestamp>,
}

/// What a step does, as a word in small capitals names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    Read,
    Searched,
    Ran,
    Called,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Read, Kind::Searched, Kind::Ran, Kind::Called];

    pub fn word(self) -> &'static str {
        match self {
            Kind::Read => "Read",
            Kind::Searched => "Searched",
            Kind::Ran => "Ran",
            Kind::Called => "Called",
        }
    }
}

impl Glance {
    pub fn count(&self, kind: Kind) -> usize {
        self.steps.iter().filter(|one| one.kind == kind).count()
    }

    pub fn runs(&self) -> impl Iterator<Item = &Run> {
        self.steps.iter().filter_map(|one| one.run.as_ref())
    }

    pub(super) fn run_mut(&mut self, id: &str) -> Option<&mut Run> {
        let runs = self.steps.iter_mut().filter_map(|one| one.run.as_mut());
        runs.into_iter().find(|one| one.id == id)
    }

    /// How long step `at` took: until the next began, or until the glance's last record.
    pub fn took(&self, at: usize) -> Option<std::time::Duration> {
        let since = self.steps.get(at)?.at?;
        let next = self.steps.get(at + 1).and_then(|one| one.at).or(self.until)?;
        Some(since.age_at(next))
    }
}

/// A shell command or an MCP tool's call, and what it gave back once it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub id: String,
    /// The MCP tool called; none for a shell command.
    pub tool: Option<String>,
    /// The command, or what the tool was given.
    pub command: String,
    pub output: Option<String>,
    pub failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subagent {
    pub id: String,
    /// Its type, as `Explore`.
    pub kind: String,
    /// The task it was given, in a few words.
    pub task: String,
    /// Its own steps, as the stream tells them.
    pub steps: Glance,
    pub answer: Option<String>,
    pub state: Sent,
    /// Its own id, when it works in the background.
    pub agent: Option<String>,
    pub since: Option<Timestamp>,
    pub until: Option<Timestamp>,
}

/// Where a subagent stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sent {
    Working,
    Done,
    Failed,
    /// Its turn was interrupted before it answered.
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub detail: Option<String>,
    /// `None` while it runs, then whether it failed.
    pub failed: Option<bool>,
}

impl Entry {
    /// Whether the tool call `id` is one of its own.
    pub(super) fn answers(&self, id: &str) -> bool {
        match self {
            Entry::Glance(glance) => glance.runs().any(|one| one.id == id),
            Entry::Edit(one) => one.id == id,
            Entry::Tool(one) => one.id == id,
            Entry::Subagent(one) => one.id == id,
            Entry::You(_) | Entry::Said(_) | Entry::Interrupted | Entry::Failed(_) => false,
        }
    }
}

impl Run {
    /// The MCP tool's own name, without its server's; none for a shell command.
    pub fn tool_name(&self) -> Option<&str> {
        self.tool.as_deref().map(short)
    }
}

/// An MCP tool's name past its `mcp__<server>__` prefix.
pub(super) fn short(name: &str) -> &str {
    name.rsplit("__").next().unwrap_or(name)
}

/// What a tool call gave back.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub failed: bool,
    /// The last lines a command printed.
    pub output: Option<String>,
    pub patch: Option<Patch>,
}

/// What a write changed: the lines gained and lost, and the first new-side line of its first hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Patch {
    pub added: u32,
    pub removed: u32,
    pub line: u32,
}

/// A command the agent's prompt takes after `/`: a skill, a plugin's or its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slash {
    pub name: String,
    pub about: String,
}

/// How much of the plan's windows is used, from 0 to 1.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Limits {
    pub five_hour: Option<Window>,
    pub week: Option<Window>,
}

/// One window of the plan: how much of it is used, from 0 to 1, and when it starts again.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Window {
    pub used: f32,
    pub resets: Option<Timestamp>,
}
