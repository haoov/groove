//! The delivery capability: what the forge says of each worktree, its threads included.

mod facts;
pub use facts::Clock;

/// The decoration on what the agent posts.
pub const BY_AGENT: &str = "agent";
mod held;
mod notes;
mod poll;
mod propose;
mod service;
mod state;

#[cfg(test)]
mod tests;

pub use groove_browser::browse;
pub use groove_forge::{Github, Remote, Snapshot, Token};
pub use held::Held;
pub use notes::listed;
pub use poll::Polling;
pub use propose::{Text, text_of};
pub use service::{Connect, Delivered, Said, Service};
pub use state::{Line, MrAct, State};
