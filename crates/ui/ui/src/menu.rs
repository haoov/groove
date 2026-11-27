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
    /// What one slot of a source's mapping can take, from its Settings row.
    Mapping(views::settings::rows::Choices),
    /// A pod's shells, one per container when it has several; a resource's writes, each asked on its row first, then the skills of its kind.
    Resource {
        session: groove_types::SessionId,
        shells: Vec<Shelled>,
        writes: Vec<crate::Losing>,
        offered: Vec<Offer>,
    },
    /// The hues a context can take, from its dot in Settings.
    Hues(String),
    /// Where a pod's logs start, from the logs view's range picker.
    LogRanges,
    /// An object open in the Kubernetes sidebar, by its place.
    Object(usize),
    /// The modes Diff's changed files, listed flat while `true`, else as a tree.
    Listing(bool),
    /// A session's row on the rail, with the section it stands in; none for a child.
    Session {
        session: groove_types::SessionId,
        section: Option<groove_types::Section>,
    },
    /// The models the agent switches to, from its row in the sidebar.
    Models {
        session: groove_types::SessionId,
        rows: Vec<Told>,
    },
}

/// One row of the models' menu: a model to switch to, marked while current.
#[derive(Debug, Clone, PartialEq)]
pub struct Told {
    pub label: String,
    pub model: Option<String>,
    pub current: bool,
}

/// One gauge of the agent's usage: what is used, how much, its share, and when it starts again.
#[derive(Debug, Clone, PartialEq)]
pub struct Gauge {
    pub label: &'static str,
    pub value: String,
    pub used: f32,
    pub resets: Option<String>,
}

/// One skill a menu row stands for, with what it is sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub id: String,
    pub args: Option<String>,
    pub label: String,
}

/// A shell a pod's row opens, in one container or the one kubectl picks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shelled {
    pub pod: Box<groove_types::Target>,
    pub container: Option<String>,
    pub label: String,
}
