//! The delivery capability: what the forge says of each worktree, and the session's notes.

mod facts;
pub use facts::Clock;

/// Who a note says left it: the user, or the agent.
pub const BY_USER: &str = "you";
pub const BY_AGENT: &str = "agent";
mod held;
mod notes;
mod poll;
mod propose;
mod service;
mod state;

#[cfg(test)]
mod tests;

pub use groove_annotations::New as NewNote;
pub use groove_browser::browse;
pub use groove_forge::{Github, Remote, Snapshot, Token};
pub use held::Held;
pub use notes::merged;
pub use poll::Polling;
pub use propose::{Text, text_of};
pub use service::{Connect, Delivered, Said, Service};
pub use state::{Line, MrAct, State};
