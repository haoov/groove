//! The window's regions, from the tokens, the window's size and the user's drags.
//!
//! Four columns full height: the rail, the agent's pane, the workspace and the
//! sidebar. The session header is the workspace's first line.

use groove_gfx::{CellSize, Rect, Size};

use crate::Ui;
use crate::ctx::Metrics;
use crate::tokens::{AGENT_MIN, RAIL_MIN, SIDEBAR_MIN, Tokens, WORKSPACE_MIN};

/// A boundary between two columns, which the user drags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// Between the rail and the agent pane.
    Rail,
    /// Between the agent pane and the workspace.
    Agent,
    /// Between the workspace and the sidebar.
    Sidebar,
}

impl Edge {
    /// Every boundary, left to right.
    pub const ALL: [Edge; 3] = [Edge::Rail, Edge::Agent, Edge::Sidebar];
}

/// Every column's width but the workspace's, in logical pixels. The workspace takes
/// what is left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Split {
    pub rail: f32,
    pub agent: f32,
    pub sidebar: f32,
}

impl Default for Split {
    fn default() -> Self {
        let tokens = Tokens::default();
        Self {
            rail: tokens.rail,
            agent: tokens.agent,
            sidebar: tokens.sidebar,
        }
    }
}

impl Split {
    /// Puts `edge` at `x`, moving only the two columns it stands between, and no
    /// column under its minimum. Logical pixels throughout.
    pub fn drag(&mut self, edge: Edge, x: f32, width: f32, sidebar: bool) {
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
        }
    }

    pub fn edge_at(&self, edge: Edge, width: f32, sidebar: bool) -> f32 {
        match edge {
            Edge::Rail => self.rail,
            Edge::Agent => self.rail + self.agent,
            Edge::Sidebar => width - self.aside(sidebar),
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
    /// The workspace's first line: what the session is, and what it points at.
    pub header: Rect,
    /// Under the header: the tabs and the tab.
    pub workspace: Rect,
    /// The tab's own list, folded to nothing when the tab has none.
    pub sidebar: Rect,
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
        Self {
            window,
            rail: Rect::new(0.0, 0.0, rail, window.h),
            agent: Rect::new(rail, 0.0, agent, window.h),
            header: Rect::new(work_x, 0.0, work_width, tokens.header),
            workspace: Rect::new(work_x, tokens.header, work_width, window.h - tokens.header),
            sidebar: Rect::new(work_x + work_width, 0.0, aside, window.h),
        }
    }

    pub fn of(metrics: Metrics, ui: &Ui) -> Self {
        let tokens = Tokens::new(metrics.scale);
        Self::new(metrics.size, &tokens, ui.split, ui.session.sidebar())
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
