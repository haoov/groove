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
    /// How many messages back it holds, while it holds that message unedited.
    pub recalled: Option<usize>,
}

impl PartialEq for Prompt {
    fn eq(&self, other: &Self) -> bool {
        let held = |one: &Self| (one.buffer.text(), one.buffer.selections().to_vec());
        let rest = |one: &Self| (one.scroll, one.follow, one.recalled);
        held(self) == held(other) && rest(self) == rest(other)
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
        (self.follow, self.recalled) = (true, None);
    }

    /// A message sent before, `back` from the newest; none empties it.
    pub fn recall(&mut self, said: Option<(usize, &str)>) {
        *self = Self::default();
        if let Some((back, text)) = said {
            self.edit(&Edit::Insert(text.replace('\r', "")));
            self.recalled = Some(back);
        }
    }

    /// The wheel moved its rows.
    pub fn scrolled(&mut self, to: f32) {
        (self.scroll, self.follow) = (to, false);
    }
}
