use groove_gfx::{Rect, Size};

const RAIL_WIDTH: f32 = 220.0;
const HEADER_HEIGHT: f32 = 32.0;
const ROW_HEIGHT: f32 = 26.0;

/// The fixed splits of the window, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub scale: f32,
    pub rail: Rect,
    pub header: Rect,
    pub body: Rect,
    pub row: f32,
}

impl Layout {
    pub fn new(size: Size, scale: f32) -> Self {
        let (w, h) = (size.width as f32, size.height as f32);
        let rail_w = RAIL_WIDTH * scale;
        let header_h = HEADER_HEIGHT * scale;
        Self {
            scale,
            rail: Rect::new(0.0, 0.0, rail_w, h),
            header: Rect::new(rail_w, 0.0, w - rail_w, header_h),
            body: Rect::new(rail_w, header_h, w - rail_w, h - header_h),
            row: ROW_HEIGHT * scale,
        }
    }

    pub fn px(&self, logical: f32) -> f32 {
        logical * self.scale
    }
}
