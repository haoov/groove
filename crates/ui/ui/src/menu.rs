//! An open menu: what it belongs to, and what it offers.

use groove_ui_kit::widgets::Corner;

use crate::views;

/// An open menu: what it belongs to, at the point it was asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    /// The corner named by `corner`, in physical pixels.
    pub at: (f32, f32),
    pub corner: Corner,
    pub of: Of,
}

/// What a menu offers.
#[derive(Debug, Clone, PartialEq)]
pub enum Of {
    /// One file of the list.
    File(String),
    /// One open file's tab, whether it owes the disk, and the other tabs that do not.
    Tab {
        path: String,
        owes: bool,
        others: Vec<String>,
    },
    /// The lines a note would stand on, from a click in the rows.
    Line { path: String, lines: (u32, u32) },
    /// One path of the explorer; `dir` while it is a directory.
    Path { path: String, dir: bool },
    /// The worktree, from the commit box: `mr` with one to write, `review` in a review.
    Worktree { mr: bool, review: bool },
    /// The session, from the header's own actions.
    Session(groove_types::SessionId),
    /// What one slot of a source's mapping can take, from its Settings row.
    Mapping(views::settings::rows::Choices),
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
