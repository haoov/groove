//! The surfaces: `view` draws the state into a `Frame`, `input` turns keys and clicks into commands.

mod components;
mod ctx;
mod editor;
mod hit;
pub mod input;
pub mod keymap;
mod layout;
mod menu;
mod offsets;
pub mod palette;
mod render;
mod settle;
mod ui;
mod views;

#[cfg(test)]
mod tests;

pub use groove_ui_kit::base::ctx::Metrics;
pub use groove_ui_kit::base::mark::Mark;
pub use groove_ui_kit::base::style::Role;
pub use groove_ui_kit::base::tokens::{SPINNER_MS, Tokens};
pub use groove_ui_kit::widgets::Corner;
pub use hit::{Cursor, Hits, Target};
pub use layout::{Edge, Split};
pub use menu::{Menu, Of, Offer};
pub use render::{ages_due, code_size, frame_commands, spins, view};
pub use ui::{AgentUi, Click, Grid, Held, Losing, Overlay, Ui};
pub use views::board::BoardUi;
pub use views::rail::RailUi;
pub use views::session::{Asked, Naming, SessionUi, Tab};

/// What the window shows beside the rail.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    #[default]
    Session,
    Board,
}

/// Which pane the keyboard belongs to.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Rail,
    #[default]
    Agent,
    Workspace,
    /// The manual section's selected terminal.
    Terminal,
    Sidebar,
}

impl Focus {
    /// The panes left to right, for a chord that moves between them.
    pub const ALL: [Focus; 4] = [Focus::Rail, Focus::Agent, Focus::Workspace, Focus::Sidebar];

    /// The pane beside this one, or this one at the edge.
    pub fn beside(self, right: bool) -> Self {
        let from = match self {
            Focus::Terminal => Focus::Workspace,
            one => one,
        };
        let at = Self::ALL.iter().position(|it| *it == from).unwrap_or(1);
        let next = match right {
            true => at + 1,
            false => at.saturating_sub(1),
        };
        Self::ALL.get(next).copied().unwrap_or(from)
    }
}

/// A boundary under the pointer: which one, and where the pointer took hold of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drag {
    pub edge: Edge,
    /// The pointer's distance from the boundary when it was grabbed, in logical pixels.
    pub offset: f32,
}
