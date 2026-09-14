//! The window's regions. Built from the tokens and the window's size, nothing else.
//!
//! Three columns full height: the rail, the agent's pane, the workspace. The session
//! header is the workspace's first line, not a band across the window.

use groove_gfx::{CellSize, Rect, Size};

use crate::tokens::{AGENT_SHARE, Tokens};

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
    pub fn new(size: Size, tokens: &Tokens) -> Self {
        let window = size.rect();
        let right = window.w - tokens.rail;
        let agent_width = (right * AGENT_SHARE).floor();
        let work_x = tokens.rail + agent_width;
        let work_width = right - agent_width;
        Self {
            window,
            rail: Rect::new(0.0, 0.0, tokens.rail, window.h),
            agent: Rect::new(tokens.rail, 0.0, agent_width, window.h),
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
