//! The agent's conversation as Groove draws it: what each transcript record says, and the rows it makes.

mod rows;

pub use rows::{Edit, Entry, Glance, Run, Tool};

use crate::Timestamp;

/// The prefix an MCP tool's name carries; its calls fold into a glance.
const MCP: &str = "mcp__";

/// The rows a conversation keeps at most; past it, the oldest quarter goes at once.
const MOST: usize = 2000;

/// What one transcript record says, in the order it says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heard {
    /// When the record was written; what follows it happened then.
    Clock(Timestamp),
    /// The user's prompt.
    You(String),
    /// The agent's words.
    Said(String),
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

/// A session's conversation, row by row.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Chat {
    entries: Vec<Entry>,
    /// Messages waiting for the agent's turn, shown after the rows.
    queued: Vec<String>,
    /// Messages sent from Groove that neither the queue nor the transcript says yet.
    sent: Vec<String>,
    /// When the last record was written.
    now: Option<Timestamp>,
}

impl Chat {
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn queued(&self) -> &[String] {
        &self.queued
    }

    pub fn sent(&self) -> &[String] {
        &self.sent
    }

    /// When the last record was written.
    pub fn now(&self) -> Option<Timestamp> {
        self.now
    }

    /// One record's say: a new row, a step folded into the glance before it, or a result on its call.
    pub fn hear(&mut self, heard: Heard) {
        let entry = match heard {
            Heard::Clock(at) => return self.clocked(at),
            Heard::You(text) => {
                self.said(&text);
                Entry::You(text)
            }
            Heard::Said(text) => Entry::Said(text),
            Heard::Interrupted => Entry::Interrupted,
            Heard::Thought => return self.glanced(|one| one.thoughts += 1),
            Heard::Look(at) => {
                return self.glanced(|one| {
                    one.doing = Some(format!("read {at}"));
                    one.looked.push(at);
                    one.running = false;
                });
            }
            Heard::Ran { id, command, about } => {
                let doing = about.unwrap_or_else(|| first_line(&command));
                return self.ran((id, None, command), doing);
            }
            Heard::Tool { id, name, detail } if name.starts_with(MCP) => {
                let doing = rows::short(&name).replace('_', " ");
                return self.ran((id, Some(name), detail.unwrap_or_default()), doing);
            }
            Heard::Queued(text) => {
                self.sent.retain(|one| *one != text);
                return self.queued.push(text);
            }
            Heard::Sent(text) => return self.sent.push(text),
            Heard::Dequeued(text) => return self.dequeued(text),
            Heard::Recalled => return self.queued.clear(),
            Heard::Edit { id, path, created } => Entry::Edit(Edit {
                id,
                path,
                created,
                patch: None,
                failed: false,
            }),
            Heard::Tool { id, name, detail } => Entry::Tool(Tool {
                id,
                name,
                detail,
                failed: None,
            }),
            Heard::Result { id, outcome } => return self.answered(&id, outcome),
        };
        self.push(entry);
    }

    fn ran(&mut self, (id, tool, command): (String, Option<String>, String), doing: String) {
        let run = Run {
            id,
            tool,
            command,
            output: None,
            failed: false,
        };
        self.glanced(|one| {
            one.ran.push(run);
            (one.running, one.doing) = (true, Some(doing));
        });
    }

    fn clocked(&mut self, at: Timestamp) {
        self.now = Some(at);
        if let Some(Entry::Glance(glance)) = self.entries.last_mut() {
            glance.until = Some(at);
        }
    }

    fn glanced(&mut self, add: impl FnOnce(&mut Glance)) {
        if !matches!(self.entries.last(), Some(Entry::Glance(_))) {
            let (since, until) = (self.now, self.now);
            self.push(Entry::Glance(Glance {
                since,
                until,
                ..Glance::default()
            }));
        }
        if let Some(Entry::Glance(glance)) = self.entries.last_mut() {
            add(glance);
        }
    }

    /// The user's words in the transcript: the waiting message saying them goes, else the oldest sent.
    fn said(&mut self, text: &str) {
        if let Some(at) = self.queued.iter().position(|one| one == text) {
            self.queued.remove(at);
        } else if let Some(at) = self.sent.iter().position(|one| one == text) {
            self.sent.remove(at);
        } else if !self.sent.is_empty() {
            self.sent.remove(0);
        }
    }

    fn dequeued(&mut self, text: Option<String>) {
        let at = match text {
            Some(text) => self.queued.iter().position(|one| *one == text),
            None => (!self.queued.is_empty()).then_some(0),
        };
        if let Some(at) = at {
            self.queued.remove(at);
        }
    }

    fn answered(&mut self, id: &str, outcome: Outcome) {
        let Some(entry) = self.entries.iter_mut().rev().find(|one| one.answers(id)) else {
            return;
        };
        match entry {
            Entry::Edit(edit) => (edit.patch, edit.failed) = (outcome.patch, outcome.failed),
            Entry::Tool(tool) => tool.failed = Some(outcome.failed),
            Entry::Glance(glance) => {
                if let Some(run) = glance.ran.iter_mut().find(|one| one.id == id) {
                    (run.output, run.failed) = (outcome.output, outcome.failed);
                }
            }
            _ => {}
        }
    }

    fn push(&mut self, entry: Entry) {
        if self.entries.len() >= MOST {
            self.entries.drain(..MOST / 4);
        }
        self.entries.push(entry);
    }
}

fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().to_string()
}
