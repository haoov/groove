//! The surfaces: `view` draws the state into a `Frame`, `input` turns keys and clicks into commands.

use groove_controllers::AppState;

mod ctx;
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
pub use views::board::BoardUi;
pub use views::session::{Asked, Naming, Scope, SessionUi, Tab};
pub use views::shared::rail::RailUi;

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
    /// Which surface the window is showing.
    pub surface: Surface,
    pub focus: Focus,
    pub palette: Option<palette::Palette>,
    pub session: SessionUi,
    pub rail: RailUi,
    pub board: BoardUi,
    pub agent: AgentUi,
    pub split: Split,
    pub drag: Option<Drag>,
    /// The pointer is down on the open file, choosing what to hold.
    pub selecting: bool,
    /// The pointer is down on the change map, dragging the lens.
    pub mapping: bool,
    /// The last press, for the next one to know whether it carries on the same click.
    pub clicked: Option<Click>,
    /// What is asking before it throws a change away.
    pub discarding: Option<Losing>,
    /// What the right button opened, and where.
    pub menu: Option<Menu>,
    /// The write the review sheet shows.
    pub examining: Option<groove_types::ApprovalId>,
    /// What the pointer is over, for the row under it to say so, and where it stands.
    pub hover: Option<Target>,
    pub at: (f32, f32),
    /// The colours the last frames read, kept while they still hold.
    pub painted: painted::Painted,
}

/// What the pointer is doing to the agent's screen.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct AgentUi {
    /// Wheel pixels not yet worth a line.
    pub carried: f32,
    /// The pointer holds a selection of our own.
    pub selecting: bool,
    /// The pointer is down, and the program in the screen is sent the reports.
    pub clicking: bool,
    /// Shift was held: the selection is ours even where the program reads the mouse.
    pub bypassed: bool,
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
    BottomLeft,
    BottomRight,
}

/// What a menu offers.
#[derive(Debug, Clone, PartialEq)]
pub enum Of {
    /// One file of the list.
    File(String),
    /// The lines a note would stand on, from a click in the rows.
    Line { path: String, lines: (u32, u32) },
    /// One path of the explorer; `dir` while it is a directory.
    Path { path: String, dir: bool },
    /// The worktree, from the commit box: `mr` with one to write, `review` in a review.
    Worktree { mr: bool, review: bool },
    /// The session, from the header's own actions.
    Session(groove_types::SessionId),
    /// The skills this session can be sent, from the agent's own bar.
    Skills {
        session: groove_types::SessionId,
        offered: Vec<Offer>,
    },
}

/// One skill a menu row stands for, with what it is sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub id: String,
    pub args: Option<String>,
    pub label: String,
}

/// What is asked before a change is thrown away.
#[derive(Debug, Clone, PartialEq)]
pub enum Losing {
    File(String),
    /// One path of the explorer, with everything under it.
    Path(String),
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
    /// The surface the window shows. With no session open, the board is the window.
    pub fn showing(&self, app: &AppState) -> Surface {
        match app.session.open.is_empty() {
            true => Surface::Board,
            false => self.surface,
        }
    }

    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// The pointer is down on something that follows it.
    pub fn pointing(&self) -> bool {
        self.drag.is_some()
            || self.selecting
            || self.mapping
            || self.board.dragging.is_some()
            || self.agent.selecting
            || self.agent.clicking
    }
}
