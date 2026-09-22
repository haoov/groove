//! The window's regions, from the tokens, the window's size and the user's drags.
//!
//! Four columns full height: the rail, the agent's pane, the workspace and the
//! sidebar. The session header is the workspace's two first lines: the title, then
//! what the session points at.

use groove_gfx::{CellSize, Rect, Size};
use groove_types::Panes;

use crate::Ui;
use crate::ctx::Metrics;
use crate::tokens::{
    AGENT_MIN, BAND_MIN, COLUMNS_MIN, COMMIT_MIN, FEED_MIN, FILES_MIN, MESSAGE_LINES, RAIL_MIN,
    SESSIONS_MIN, SIDEBAR_MIN, Tokens, WORKSPACE_MIN,
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
    /// Between the board's columns and the timeline under them.
    Band,
    /// Between the rail's own rows and the feed under them.
    Feed,
}

impl Edge {
    /// Every boundary: the three columns, then the one across the sidebar.
    pub const ALL: [Edge; 4] = [Edge::Rail, Edge::Agent, Edge::Sidebar, Edge::Commit];

    /// Whether the boundary is a vertical line, which the pointer moves sideways.
    pub fn upright(self) -> bool {
        !matches!(self, Edge::Commit | Edge::Band | Edge::Feed)
    }
}

/// Every column's width but the workspace's, in logical pixels. The workspace takes
/// what is left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Split {
    pub rail: f32,
    pub agent: f32,
    pub sidebar: f32,
    /// How tall the commit box stands at the sidebar's foot.
    pub commit: f32,
    /// How tall the board's timeline stands under its columns.
    pub band: f32,
    /// How tall the rail's feed stands, the footer under it included.
    pub feed: f32,
}

impl Default for Split {
    fn default() -> Self {
        let tokens = Tokens::default();
        Self {
            rail: tokens.rail,
            agent: tokens.agent,
            sidebar: tokens.sidebar,
            commit: tokens.row + tokens.line * MESSAGE_LINES as f32,
            band: tokens.band,
            feed: FEED_MIN * 2.0,
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
            band: panes.band.max(BAND_MIN),
            feed: panes.feed.max(FEED_MIN),
        }
    }

    pub fn panes(&self) -> Panes {
        Panes {
            rail: self.rail,
            agent: self.agent,
            sidebar: self.sidebar,
            commit: self.commit,
            band: self.band,
            feed: self.feed,
        }
    }

    /// Puts `edge` at `x`, moving only the two columns it stands between, and no
    /// column under its minimum. Logical pixels throughout.
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
            Edge::Band => {
                let most = (height - COLUMNS_MIN).max(BAND_MIN);
                self.band = (height - at).clamp(BAND_MIN, most);
            }
            Edge::Feed => {
                let most = (height - SESSIONS_MIN).max(FEED_MIN);
                self.feed = (height - at).clamp(FEED_MIN, most);
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
            Edge::Band => height - self.band,
            Edge::Feed => height - self.feed,
        }
    }

    fn room(&self, width: f32, sidebar: bool) -> f32 {
        (width - self.rail - self.aside(sidebar)).max(1.0)
    }

    fn aside(&self, sidebar: bool) -> f32 {
        match sidebar {
            true => self.sidebar,
            false => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub window: Rect,
    pub rail: Rect,
    pub agent: Rect,
    /// The workspace's two first lines: the title, then what it points at.
    pub header: Rect,
    /// Under the header: the tabs and the tab.
    pub workspace: Rect,
    /// The tab's own list, folded to nothing when the tab has none.
    pub sidebar: Rect,
    /// The commit box at the sidebar's foot, as tall as the user has dragged it.
    pub commit: Rect,
    /// The rail's feed, above its footer, as tall as the user has dragged it.
    pub feed: Rect,
    /// Everything the board takes: the window but the rail.
    pub board: Rect,
}

impl Layout {
    pub fn new(size: Size, tokens: &Tokens, split: Split, sidebar: bool) -> Self {
        let window = size.rect();
        let scale = |logical: f32| (logical * tokens.scale).floor();
        let rail = scale(split.rail);
        let agent = scale(split.agent);
        let aside = scale(split.aside(sidebar));
        let work_x = rail + agent;
        let work_width = (window.w - work_x - aside).max(0.0);
        let box_ = scale(split.commit).min(window.h);
        let foot = tokens.row;
        let band = (scale(split.feed).min(window.h) - foot).max(tokens.row);
        let head = tokens.header + tokens.row + tokens.sm;
        Self {
            window,
            feed: Rect::new(0.0, window.h - foot - band, rail, band),
            commit: Rect::new(work_x + work_width, window.h - box_, aside, box_),
            rail: Rect::new(0.0, 0.0, rail, window.h),
            agent: Rect::new(rail, 0.0, agent, window.h),
            header: Rect::new(work_x, 0.0, work_width, head),
            workspace: Rect::new(work_x, head, work_width, window.h - head),
            sidebar: Rect::new(work_x + work_width, 0.0, aside, window.h),
            board: Rect::new(rail, 0.0, (window.w - rail).max(0.0), window.h),
        }
    }

    pub fn of(metrics: Metrics, ui: &Ui) -> Self {
        let tokens = metrics.tokens();
        let mut held = Self::new(metrics.size, &tokens, ui.split, ui.session.sidebar());
        if ui.rail.folded {
            let row = tokens.row;
            held.feed = Rect::new(0.0, held.feed.bottom() - row, held.feed.w, row);
        }
        if !ui.session.commits() {
            held.commit = Rect::new(held.commit.x, held.window.h, held.commit.w, 0.0);
        }
        held
    }

    /// Where the agent's grid starts inside its pane.
    pub fn agent_origin(&self, tokens: &Tokens) -> (f32, f32) {
        (self.agent.x + tokens.sm, self.agent.y + tokens.sm)
    }

    /// The columns and rows the agent pane holds at this cell size.
    pub fn agent_grid(&self, tokens: &Tokens, cell: CellSize) -> (u16, u16) {
        let pad = tokens.sm * 2.0;
        let cols = ((self.agent.w - pad) / cell.width).floor().max(1.0);
        let rows = ((self.agent.h - pad) / cell.height).floor().max(1.0);
        (cols as u16, rows as u16)
    }
}
