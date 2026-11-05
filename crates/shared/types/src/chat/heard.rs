//! What one record of the agent says, before the conversation makes rows of it.

use super::{Outcome, Sent};
use crate::Timestamp;

/// What one transcript record says, in the order it says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heard {
    /// When the record was written; what follows it happened then.
    Clock(Timestamp),
    /// The user's prompt.
    You(String),
    /// The agent's words.
    Said(String),
    /// The model that answered, and how many tokens its context held.
    Used {
        model: String,
        context: u64,
    },
    /// A piece of the agent's words as it writes them, before the transcript holds them whole.
    Writing(String),
    Thought,
    /// A tool that only looks, and what it looked at.
    Look(String),
    Edit {
        id: String,
        path: String,
        created: bool,
    },
    /// A shell command run, and what the agent said it does.
    Ran {
        id: String,
        command: String,
        about: Option<String>,
    },
    /// A message typed while the agent works, waiting for its turn.
    Queued(String),
    /// A waiting message taken: the one that says this, else the first.
    Dequeued(Option<String>),
    /// Every waiting message taken back to be edited.
    Recalled,
    /// Words sent from Groove's own prompt, before the transcript says them.
    Sent(String),
    /// The user stopped the agent's turn.
    Interrupted,
    /// An agent sent with a task, by its type.
    Subagent {
        id: String,
        kind: String,
        task: String,
    },
    /// A subagent sent to work in the background, by its own id.
    Background {
        id: String,
        agent: String,
    },
    /// A background subagent ended, named by its call or its own id, with what it said.
    Ended {
        call: Option<String>,
        agent: String,
        state: Sent,
        summary: Option<String>,
    },
    /// A background subagent's report, by its own id.
    Reported {
        agent: String,
        report: String,
    },
    /// What the subagent `parent` did, in its own records.
    Within {
        parent: String,
        heard: Vec<Heard>,
    },
    /// An API error, a reached limit, or the agent ending by itself.
    Failed(String),
    Tool {
        id: String,
        name: String,
        detail: Option<String>,
    },
    /// What came back from the tool call `id`.
    Result {
        id: String,
        outcome: Outcome,
    },
}
