//! Groove's own prompt: the words typed for the agent, and how far its rows are scrolled.

use groove_controllers::workspace_service::Buffer;
use groove_types::Edit;

#[derive(Debug, Default, Clone)]
pub struct Prompt {
    pub buffer: Buffer,
    /// How far its rows are scrolled down, in pixels.
    pub scroll: f32,
    /// The rows follow the caret, until the wheel moves them.
    pub follow: bool,
}

impl PartialEq for Prompt {
    fn eq(&self, other: &Self) -> bool {
        let held = |one: &Self| (one.buffer.text(), one.buffer.selections().to_vec());
        held(self) == held(other) && (self.scroll, self.follow) == (other.scroll, other.follow)
    }
}

impl Prompt {
    pub fn text(&self) -> String {
        self.buffer.text()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.lines() <= 1 && self.buffer.line(0).is_none_or(|one| one.is_empty())
    }

    /// One edit, the caret kept in sight.
    pub fn edit(&mut self, edit: &Edit) {
        self.buffer.edit(edit);
        self.follow = true;
    }

    /// The wheel moved its rows.
    pub fn scrolled(&mut self, to: f32) {
        (self.scroll, self.follow) = (to, false);
    }
}
