//! A reply being typed under a thread of the MR, and what its words become.

use groove_controllers::delivery::{self, ThreadAct};
use groove_controllers::workspace_service::Buffer;
use groove_types::Anchor;

/// A reply being typed in the surface: the lines its thread hangs on, the thread, its words.
#[derive(Debug, Clone)]
pub struct Noting {
    pub anchor: Anchor,
    pub buffer: Buffer,
    pub thread: String,
}

impl PartialEq for Noting {
    fn eq(&self, other: &Self) -> bool {
        let held = |one: &Self| (one.buffer.text(), one.buffer.selections().to_vec());
        (&self.anchor, &self.thread) == (&other.anchor, &other.thread) && held(self) == held(other)
    }
}

impl Eq for Noting {}

impl Noting {
    /// An empty row under a thread, for the words that answer it.
    pub fn reply(anchor: Anchor, thread: String) -> Self {
        Self {
            anchor,
            buffer: Buffer::default(),
            thread,
        }
    }

    /// What the reply says, with nothing around it.
    pub fn said(&self) -> String {
        self.buffer.text().trim().to_string()
    }

    /// The reply sent under its thread, or nothing while it says nothing.
    pub fn sent(&self) -> Option<delivery::Command> {
        let body = self.said();
        if body.is_empty() {
            return None;
        }
        let thread = self.thread.clone();
        Some(delivery::Command::Thread(ThreadAct::Reply { thread, body }))
    }
}
