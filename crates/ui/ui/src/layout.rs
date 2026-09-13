use groove_gfx::{CellSize, Rect, Size};

const RAIL_WIDTH: f32 = 220.0;
const HEADER_HEIGHT: f32 = 32.0;
const ROW_HEIGHT: f32 = 26.0;
const PANE_PAD: f32 = 8.0;
/// The agent pane's share of the body.
const AGENT_SHARE: f32 = 0.45;

/// The fixed splits of the window, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub scale: f32,
    pub rail: Rect,
    pub header: Rect,
    pub body: Rect,
    pub agent: Rect,
    pub workspace: Rect,
    pub row: f32,
}

impl Layout {
    pub fn new(size: Size, scale: f32) -> Self {
        let (w, h) = (size.width as f32, size.height as f32);
        let rail_w = RAIL_WIDTH * scale;
        let header_h = HEADER_HEIGHT * scale;
        let body = Rect::new(rail_w, header_h, w - rail_w, h - header_h);
        let agent_w = (body.w * AGENT_SHARE).floor();
        Self {
            scale,
            rail: Rect::new(0.0, 0.0, rail_w, h),
            header: Rect::new(rail_w, 0.0, w - rail_w, header_h),
            body,
            agent: Rect::new(body.x, body.y, agent_w, body.h),
            workspace: Rect::new(body.x + agent_w, body.y, body.w - agent_w, body.h),
            row: ROW_HEIGHT * scale,
        }
    }

    pub fn px(&self, logical: f32) -> f32 {
        logical * self.scale
    }

    /// Where the agent's grid starts inside its pane.
    pub fn agent_origin(&self) -> (f32, f32) {
        (
            self.agent.x + self.px(PANE_PAD),
            self.agent.y + self.px(PANE_PAD),
        )
    }

    /// The columns and rows the agent pane holds at this cell size.
    pub fn agent_grid(&self, cell: CellSize) -> (u16, u16) {
        let pad = self.px(PANE_PAD);
        let cols = ((self.agent.w - 2.0 * pad) / cell.width).floor().max(1.0);
        let rows = ((self.agent.h - 2.0 * pad) / cell.height).floor().max(1.0);
        (cols as u16, rows as u16)
    }
}
