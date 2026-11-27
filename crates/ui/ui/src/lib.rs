//! The surfaces: `view` draws the state into a `Frame`, `input` turns keys and clicks into commands.

mod components;
mod ctx;
mod editor;
mod hit;
pub mod input;
pub mod keymap;
mod layout;
mod menu;
mod motion;
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
pub use groove_ui_kit::widgets::NEURON_MS;
pub use hit::{Cursor, Hits, Target};
pub use layout::{Edge, Split};
pub use menu::{Menu, Of, Offer, Shelled};
pub use motion::{MOTION_MS, Motion};
pub use render::{ages_due, code_size, fires, frame_commands, spins, view};
pub use ui::{AgentUi, Click, Grid, Held, Losing, Overlay, Ui};
pub use views::board::BoardUi;
pub use views::rail::RailUi;
pub use views::session::{Asked, Mode, Naming, SessionUi};

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
    /// The middle, on the code or the cluster's list.
    Workspace,
    /// The manual section's selected terminal.
    Terminal,
    Sidebar,
}

/// A boundary under the pointer: which one, and where the pointer took hold of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drag {
    pub edge: Edge,
    /// The pointer's distance from the boundary when it was grabbed, in logical pixels.
    pub offset: f32,
}
