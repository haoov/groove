//! Where a drawn table's rows and cells stand.

use groove_gfx::Rect;

/// Where a table's rows stand once drawn.
pub struct Laid {
    /// How far the rows scroll, in pixels.
    pub extent: f32,
    pub spans: Vec<(f32, f32)>,
    /// The rows' room, under the header.
    pub body: Rect,
    pub(super) offset: f32,
    pub(super) height: f32,
    pub(super) first: f32,
}

impl Laid {
    pub(super) fn line(&self, row: usize) -> Rect {
        let y = self.body.y - self.offset + row as f32 * self.height;
        Rect::new(self.body.x, y, self.body.w, self.height)
    }

    /// The first line of row `row` in column `column`.
    pub fn cell(&self, row: usize, column: usize) -> Rect {
        let (x, w) = self.spans[column];
        Rect::new(x, self.line(row).y, w, self.first)
    }

    /// The first line of row `row` from column `column`'s left edge to the end.
    pub fn from(&self, row: usize, column: usize) -> Rect {
        let (x, _) = self.spans[column];
        Rect::new(x, self.line(row).y, self.body.right() - x, self.first)
    }
}

/// The first row on screen and the one past the last.
pub(super) fn visible(offset: f32, room: f32, height: f32, count: usize) -> (usize, usize) {
    if height <= 0.0 {
        return (0, 0);
    }
    let first = (offset / height).floor() as usize;
    let last = ((offset + room) / height).ceil() as usize + 1;
    (first.min(count), last.min(count))
}
