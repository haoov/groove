//! What the pointer says over a target, and what answers the agent's ask.

use super::{Cursor, Target};

impl Target {
    /// What the pointer says over it: a row is a pointer, and these are not.
    pub(in crate::hit) fn cursor(&self) -> Cursor {
        match self {
            Target::Term(_) | Target::Finding | Target::Message | Target::Code | Target::Prompt => {
                Cursor::Text
            }
            Target::Agent
            | Target::Chat
            | Target::Shell(_)
            | Target::Pinned
            | Target::Palette
            | Target::Overview
            | Target::Listing => Cursor::Default,
            Target::Map | Target::Place(_) => Cursor::RowResize,
            Target::ResourceEdge(_) | Target::AlertEdge(_) => Cursor::ColResize,
            Target::Split(edge) => match edge.upright() {
                true => Cursor::ColResize,
                false => Cursor::RowResize,
            },
            _ => Cursor::Pointer,
        }
    }
}

/// What answers a headless agent's ask: leave to run the tool, a refusal, or one option of its question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answering {
    Allow,
    Refuse,
    Pick(String),
    /// The question before, its pick undone.
    Back,
}
