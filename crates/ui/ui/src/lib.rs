//! The surfaces. `view` reads `AppState` and the ui's own state into a `Frame`;
//! `input` turns a key or a click into a `Command`. Nothing here talks to a service.

pub mod input;
pub mod layout;
mod theme;
mod view;
mod widget;

#[cfg(test)]
mod tests;

pub use theme::theme;
pub use view::{Metrics, layout_commands, view};

/// Which pane the keyboard belongs to.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Agent,
    Rail,
}

/// What is the ui's alone: focus, folds, the palette. Never in `AppState`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Ui {
    pub palette_open: bool,
    pub focus: Focus,
}
