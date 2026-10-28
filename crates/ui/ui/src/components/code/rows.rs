//! The rows a surface draws, from the top of their rect.

use groove_gfx::Rect;

use super::row::draw;
use super::{Block, Gutters, Line};
use crate::ctx::Ctx;

/// The rows a surface draws: the window it built, and the gutter they share.
pub struct Rows<'a> {
    pub lines: &'a [Line<'a>],
    /// Where the first line sits among all the rows.
    pub first: usize,
    pub gutters: Gutters,
    /// How far the text is scrolled sideways, its gutters left in place.
    pub across: f32,
    /// Whether this pass draws the floating rows of notes' cards, or every other row.
    pub floating: bool,
}

/// Rows of code from the top of `rect`, scrolled by `scroll`, clipped to it.
pub fn code(ctx: &mut Ctx, rect: Rect, rows: Rows<'_>, scroll: f32) -> Vec<Rect> {
    let height = ctx.tokens.code_line;
    let block = Block::of(ctx, rows.gutters);
    let mut drawn = Vec::with_capacity(rows.lines.len());
    ctx.clipped(rect, |ctx| {
        let mut y = rect.y - scroll + rows.first as f32 * height;
        for line in rows.lines {
            let at = groove_ui_kit::shape::band(rect, y - rect.y, height);
            if line.float == rows.floating {
                draw(ctx, at, line, block, rows.across);
            }
            drawn.push(at);
            y += height;
        }
    });
    drawn
}
