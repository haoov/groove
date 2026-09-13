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
pub use view::view;

/// What is the ui's alone: selection, folds, the palette. Never in `AppState`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Ui {
    pub palette_open: bool,
}
