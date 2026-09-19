//! A cell grid drawn as one batch: the terminal and the code surface.

use crate::fonts::CellSize;
use crate::quads::QuadPass;
use crate::text::TextPass;
use crate::{CellGrid, Fonts, Rect, Size, boxdraw};

/// Backgrounds and box glyphs as quads, the rest one glyph per cell.
pub(crate) fn push_grid(
    layer: usize,
    grid: &CellGrid,
    cell: CellSize,
    quads: &mut QuadPass,
    text: &mut TextPass,
    fonts: &mut Fonts,
    size: Size,
) {
    for (col, row, c) in grid.iter() {
        let x = grid.x + col as f32 * cell.width;
        let y = grid.y + row as f32 * cell.height;
        if !c.bg.is_transparent() {
            let rect = Rect::new(x, y, cell.width, cell.height);
            quads.push(layer, rect, c.bg, 1.0, grid.clip, size);
        }
        if c.is_blank() {
            continue;
        }
        match boxdraw::quads(c.ch, x, y, cell.width, cell.height, grid.font_size) {
            Some(boxes) => {
                for ([x0, y0, x1, y1], alpha) in boxes {
                    let rect = Rect::new(x0, y0, x1 - x0, y1 - y0);
                    quads.push(layer, rect, c.fg, alpha, grid.clip, size);
                }
            }
            None => text.push_glyph(
                layer,
                fonts,
                c.ch,
                c.bold,
                grid.font_size,
                cell,
                x,
                y,
                c.fg,
                grid.clip,
            ),
        }
    }
}
