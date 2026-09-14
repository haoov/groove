//! The window's regions. Built from the tokens and the window's size, nothing else.

use groove_gfx::{CellSize, Rect, Size};

use crate::tokens::{AGENT_SHARE, Tokens};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub window: Rect,
    pub rail: Rect,
    pub header: Rect,
    pub body: Rect,
    pub agent: Rect,
    pub workspace: Rect,
}

impl Layout {
    pub fn new(size: Size, tokens: &Tokens) -> Self {
        let window = size.rect();
        let body = Rect::new(
            tokens.rail,
            tokens.header,
            window.w - tokens.rail,
            window.h - tokens.header,
        );
        let agent_width = (body.w * AGENT_SHARE).floor();
        Self {
            window,
            rail: Rect::new(0.0, 0.0, tokens.rail, window.h),
            header: Rect::new(tokens.rail, 0.0, window.w - tokens.rail, tokens.header),
            body,
            agent: Rect::new(body.x, body.y, agent_width, body.h),
            workspace: Rect::new(body.x + agent_width, body.y, body.w - agent_width, body.h),
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
