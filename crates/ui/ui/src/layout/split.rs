//! What the user dragged: every column's width but the workspace's, and the heights across.

use groove_types::Panes;

use groove_ui_kit::base::tokens::{
    AGENT_MIN, CODE_MIN, COMMIT_MIN, FEED_MIN, FILES_MIN, MANUAL_MIN, MANUAL_TALL, MESSAGE_LINES,
    RAIL_MIN, SESSIONS_MIN, SIDEBAR_MIN, Tokens, WORKSPACE_MIN,
};

/// A boundary the user drags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// Between the rail and the agent pane.
    Rail,
    /// Between the agent pane and the workspace.
    Agent,
    /// Between the workspace and the sidebar.
    Sidebar,
    /// Between the changed files and the commit box under them.
    Commit,
    /// Between the rail's own rows and the feed under them.
    Feed,
    /// Between the workspace's tab and the manual section under it.
    Manual,
}

impl Edge {
    /// Every boundary: the three columns, then the ones across the sidebar and the workspace.
    pub const ALL: [Edge; 5] = [
        Edge::Rail,
        Edge::Agent,
        Edge::Sidebar,
        Edge::Commit,
        Edge::Manual,
    ];

    /// Whether the boundary is a vertical line, which the pointer moves sideways.
    pub fn upright(self) -> bool {
        !matches!(self, Edge::Commit | Edge::Feed | Edge::Manual)
    }
}

/// Every column's width but the workspace's, which takes the rest, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Split {
    pub rail: f32,
    pub agent: f32,
    pub sidebar: f32,
    /// How tall the commit box stands at the sidebar's foot.
    pub commit: f32,
    /// How tall the rail's feed stands, the footer under it included.
    pub feed: f32,
    /// How tall the manual section stands open.
    pub manual: f32,
}

impl Default for Split {
    fn default() -> Self {
        let tokens = Tokens::default();
        Self {
            rail: tokens.rail,
            agent: tokens.agent,
            sidebar: tokens.sidebar,
            commit: tokens.row + tokens.line * MESSAGE_LINES as f32,
            feed: FEED_MIN * 2.0,
            manual: MANUAL_TALL,
        }
    }
}

impl Split {
    /// What a past run left, no column under its minimum.
    pub fn of(panes: Panes) -> Self {
        Self {
            rail: panes.rail.max(RAIL_MIN),
            agent: panes.agent.max(AGENT_MIN),
            sidebar: panes.sidebar.max(SIDEBAR_MIN),
            commit: panes.commit.max(COMMIT_MIN),
            feed: panes.feed.max(FEED_MIN),
            manual: panes.manual.max(MANUAL_MIN),
        }
    }

    pub fn panes(&self) -> Panes {
        Panes {
            rail: self.rail,
            agent: self.agent,
            sidebar: self.sidebar,
            commit: self.commit,
            feed: self.feed,
            manual: self.manual,
        }
    }

    /// Puts `edge` at `x`, moving only its two columns and none under its minimum.
    pub fn drag(&mut self, edge: Edge, at: f32, window: (f32, f32), sidebar: bool) {
        let (width, height) = window;
        let x = at;
        match edge {
            Edge::Rail => {
                let held = self.rail + self.agent;
                self.rail = x.clamp(RAIL_MIN, (held - AGENT_MIN).max(RAIL_MIN));
                self.agent = held - self.rail;
            }
            Edge::Agent => {
                let most = (self.room(width, sidebar) - WORKSPACE_MIN).max(AGENT_MIN);
                self.agent = (x - self.rail).clamp(AGENT_MIN, most);
            }
            Edge::Sidebar => {
                let most = (width - self.rail - self.agent - WORKSPACE_MIN).max(SIDEBAR_MIN);
                self.sidebar = (width - x).clamp(SIDEBAR_MIN, most);
            }
            Edge::Commit => {
                let most = (height - FILES_MIN).max(COMMIT_MIN);
                self.commit = (height - at).clamp(COMMIT_MIN, most);
            }
            Edge::Feed => {
                let most = (height - SESSIONS_MIN).max(FEED_MIN);
                self.feed = (height - at).clamp(FEED_MIN, most);
            }
            Edge::Manual => {
                let most = (height - CODE_MIN).max(MANUAL_MIN);
                self.manual = (height - at).clamp(MANUAL_MIN, most);
            }
        }
    }

    pub fn edge_at(&self, edge: Edge, window: (f32, f32), sidebar: bool) -> f32 {
        let (width, height) = window;
        match edge {
            Edge::Rail => self.rail,
            Edge::Agent => self.rail + self.agent,
            Edge::Sidebar => width - self.aside(sidebar),
            Edge::Commit => height - self.commit,
            Edge::Feed => height - self.feed,
            Edge::Manual => height - self.manual,
        }
    }

    fn room(&self, width: f32, sidebar: bool) -> f32 {
        (width - self.rail - self.aside(sidebar)).max(1.0)
    }

    pub(super) fn aside(&self, sidebar: bool) -> f32 {
        match sidebar {
            true => self.sidebar,
            false => 0.0,
        }
    }
}
