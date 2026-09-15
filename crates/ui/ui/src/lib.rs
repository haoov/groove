//! The surfaces. `view` reads `AppState` and the ui's own state into a `Frame`;
//! `input` turns a key or a click into commands. Nothing here talks to a service.
//!
//! One directory per surface under `views/`, the surface's own file named after it.

mod ctx;
mod hit;
pub mod input;
mod layout;
mod mark;
pub mod palette;
mod render;
mod style;
mod tokens;
mod views;
mod widget;

#[cfg(test)]
mod tests;

pub use ctx::Metrics;
pub use hit::{Cursor, Hits, Target};
pub use layout::{Edge, Split};
pub use mark::Mark;
pub use render::{layout_commands, view};
pub use style::Role;
pub use tokens::Tokens;
pub use views::session::{SessionUi, Tab};

/// Which pane the keyboard belongs to.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Agent,
    Rail,
}

/// A boundary under the pointer: which one, and where the pointer took hold of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drag {
    pub edge: Edge,
    /// The pointer's distance from the boundary when it was grabbed, in logical pixels.
    pub offset: f32,
}

/// What is the ui's alone: focus, folds, the palette, the splits. Never in `AppState`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Ui {
    pub focus: Focus,
    pub palette: Option<palette::Palette>,
    pub session: SessionUi,
    pub split: Split,
    pub drag: Option<Drag>,
}

impl Ui {
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }
}
