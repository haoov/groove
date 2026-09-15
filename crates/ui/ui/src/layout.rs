//! The window's regions. Built from the tokens and the window's size, nothing else.
//!
//! Three columns full height: the rail, the agent's pane, the workspace. The session
//! header is the workspace's first line, not a band across the window.

use groove_gfx::{CellSize, Rect, Size};

use crate::tokens::{AGENT_MIN, AGENT_SHARE, RAIL_MIN, Tokens, WORKSPACE_MIN};

/// A boundary between two columns, which the user drags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// Between the rail and the agent pane.
    Rail,
    /// Between the agent pane and the workspace.
    Agent,
}

/// Where the columns divide, in logical pixels. The window's size never changes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Split {
    pub rail: f32,
    /// The agent pane's share of what the rail leaves.
    pub agent: f32,
}

impl Default for Split {
    fn default() -> Self {
        Self {
            rail: Tokens::default().rail,
            agent: AGENT_SHARE,
        }
    }
}

impl Split {
    /// Puts `edge` at `x`, and no column under its minimum. Logical pixels throughout.
    pub fn drag(&mut self, edge: Edge, x: f32, width: f32) {
        match edge {
            Edge::Rail => {
                let most = (width - AGENT_MIN - WORKSPACE_MIN).max(RAIL_MIN);
                self.rail = x.clamp(RAIL_MIN, most);
            }
            Edge::Agent => {
                let right = (width - self.rail).max(1.0);
                let most = (right - WORKSPACE_MIN).max(AGENT_MIN);
                self.agent = (x - self.rail).clamp(AGENT_MIN, most) / right;
            }
        }
    }

    /// Where `edge` stands in a window this wide, in logical pixels.
    pub fn edge_at(&self, edge: Edge, width: f32) -> f32 {
        match edge {
            Edge::Rail => self.rail,
            Edge::Agent => self.rail + (width - self.rail) * self.agent,
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
}

impl Layout {
    pub fn new(size: Size, tokens: &Tokens, split: Split) -> Self {
        let window = size.rect();
        let rail_width = (split.rail * tokens.scale).floor();
        let right = window.w - rail_width;
        let agent_width = (right * split.agent).floor();
        let work_x = rail_width + agent_width;
        let work_width = right - agent_width;
        Self {
            window,
            rail: Rect::new(0.0, 0.0, rail_width, window.h),
            agent: Rect::new(rail_width, 0.0, agent_width, window.h),
            header: Rect::new(work_x, 0.0, work_width, tokens.header),
            workspace: Rect::new(work_x, tokens.header, work_width, window.h - tokens.header),
        }
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
