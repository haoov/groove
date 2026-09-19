//! The surfaces. `view` reads `AppState` and the ui's own state into a `Frame`;
//! `input` turns a key or a click into commands. Nothing here talks to a service.
//!
//! One directory per surface under `views/`, the surface's own file named after it.

mod ctx;
mod field;
mod find;
mod hit;
pub mod input;
mod layout;
mod mark;
mod painted;
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
pub use views::shared::rail::RailUi;

/// Which pane the keyboard belongs to.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Rail,
    #[default]
    Agent,
    Workspace,
    Sidebar,
}

impl Focus {
    /// The panes left to right, for a chord that moves between them.
    pub const ALL: [Focus; 4] = [Focus::Rail, Focus::Agent, Focus::Workspace, Focus::Sidebar];

    /// The pane beside this one, or this one at the edge.
    pub fn beside(self, right: bool) -> Self {
        let at = Self::ALL.iter().position(|it| *it == self).unwrap_or(1);
        let next = match right {
            true => at + 1,
            false => at.saturating_sub(1),
        };
        Self::ALL.get(next).copied().unwrap_or(self)
    }
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
    pub rail: RailUi,
    pub split: Split,
    pub drag: Option<Drag>,
    /// The pointer is down on the open file, so it is choosing what to hold.
    pub selecting: bool,
    /// The pointer is down on the change map, so it is dragging the lens.
    pub mapping: bool,
    /// The last press, for the next one to know whether it carries on the same click.
    pub clicked: Option<Click>,
    /// What is asking before it throws a change away.
    pub discarding: Option<Losing>,
    /// What the right button opened, and where.
    pub menu: Option<Menu>,
    /// What the pointer is over, for the row under it to say so.
    pub hover: Option<Target>,
    /// The colours the last frames read, kept while they still hold.
    pub painted: painted::Painted,
}

/// An open menu: what it belongs to, at the point it was asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    /// The corner named by `corner`, in physical pixels.
    pub at: (f32, f32),
    pub corner: Corner,
    pub of: Of,
}

/// Which corner of the panel sits at the point it was opened from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Corner {
    TopLeft,
    BottomRight,
}

/// What a menu offers.
#[derive(Debug, Clone, PartialEq)]
pub enum Of {
    /// One file of the list.
    File(String),
    /// The worktree, from the commit box.
    Worktree,
}

/// What is asked before a change is thrown away.
#[derive(Debug, Clone, PartialEq)]
pub enum Losing {
    File(String),
    Everything,
}

/// A press, and how many the pointer has made in the same place in a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Click {
    pub x: f32,
    pub y: f32,
    pub at: u64,
    pub count: u32,
}

impl Ui {
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// The pointer is down on something that follows it, so its moves are input.
    pub fn pointing(&self) -> bool {
        self.drag.is_some() || self.selecting || self.mapping
    }
}
