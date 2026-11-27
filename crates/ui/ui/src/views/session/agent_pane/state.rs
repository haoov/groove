//! What the agent pane remembers between frames: the prompt, the scroll, a drag, what is open.

use crate::views;

/// The agent pane: its conversation, the prompt, and what the pointer does to them.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AgentUi {
    /// Wheel pixels not yet worth a line.
    pub carried: f32,
    /// Shift was held: the selection is ours even where the program reads the mouse.
    pub bypassed: bool,
    pub prompt: views::session::Prompt,
    /// How far the conversation is scrolled up from its end, in pixels.
    pub scroll: f32,
    /// What a drag holds of the conversation: where it started and where it is, the scroll taken out.
    pub reading: Option<((f32, f32), (f32, f32))>,
    /// The glances whose commands show whole, by their first command.
    pub opened: Vec<String>,
    /// The command `/` offers that Enter or Tab takes.
    pub offered: usize,
    /// Escape put the offers away until the prompt changes.
    pub hushed: bool,
    /// The options picked so far for the ask that id names, one per question.
    pub picked: Option<(String, Vec<String>)>,
}

impl AgentUi {
    /// The options picked for the ask `id`; none for any other.
    pub fn picked(&self, id: &str) -> Vec<String> {
        let held = self.picked.as_ref().filter(|(one, _)| one == id);
        held.map(|(_, picked)| picked.clone()).unwrap_or_default()
    }

    /// A point on the screen as one of the conversation's own, its scroll taken out.
    pub fn unscrolled(&self, (x, y): (f32, f32)) -> (f32, f32) {
        (x, y - self.scroll)
    }

    /// The ends of what a drag holds of the conversation, on the screen at its scroll.
    pub fn held(&self) -> Option<((f32, f32), (f32, f32))> {
        let ((a, b), (c, d)) = self.reading?;
        Some(((a, b + self.scroll), (c, d + self.scroll)))
    }
}
