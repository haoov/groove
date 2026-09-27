//! The delivery capability: what the forge says of each worktree, and the session's notes.

mod held;
mod notes;
mod poll;
mod propose;
mod service;
mod state;

#[cfg(test)]
mod tests;

pub use groove_annotations::New as NewNote;
pub use groove_forge::{Remote, Snapshot};
pub use held::Held;
pub use notes::merged;
pub use poll::Polling;
pub use propose::{Text, text_of};
pub use service::{Connect, Delivered, Said, Service};
pub use state::{Line, MrAct, State};
