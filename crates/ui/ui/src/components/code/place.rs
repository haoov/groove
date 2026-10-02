//! Where rows and characters stand on the surface, and what a point lands on.

use std::ops::Range;

use groove_gfx::Rect;

use super::Gutters;
use super::gutter::Block;
use crate::ctx::Ctx;
use crate::hit::Chars;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::Tokens;

/// The row at the top of a surface scrolled this far.
pub fn first(line: f32, scroll: f32) -> usize {
    (scroll / line).floor().max(0.0) as usize
}

/// Where a surface with these gutters puts its characters in `rect`, scrolled `across`.
pub fn chars_of(
    ctx: &mut Ctx,
    gutters: Gutters,
    rect: Rect,
    (scroll, across): (f32, f32),
) -> Chars {
    let style = ctx.styles.code(Role::Text);
    Chars {
        left: Block::of(ctx, gutters).content(ctx, rect) - across,
        advance: ctx.measure("M", &style),
        scroll,
    }
}

/// The rows `rect` has room for at `scroll`, among `total`.
pub fn visible(ctx: &Ctx, rect: Rect, total: usize, scroll: f32) -> Range<usize> {
    let height = ctx.tokens.line;
    let first = first(height, scroll).min(total);
    let shown = (rect.h / height).ceil() as usize + 1;
    first..(first + shown).min(total)
}

/// How tall the rows stand together.
pub fn height(ctx: &Ctx, lines: usize) -> f32 {
    ctx.tokens.line * lines as f32
}

/// How far the widest of `texts` runs past the room its text has in `rect`.
pub fn across_extent<'a>(
    ctx: &mut Ctx,
    gutters: Gutters,
    rect: Rect,
    texts: impl Iterator<Item = &'a str>,
) -> f32 {
    let style = ctx.styles.code(Role::Text);
    let advance = ctx.measure("M", &style);
    let widest = texts.map(|text| text.chars().count()).max().unwrap_or(0);
    let room = rect.right() - Block::of(ctx, gutters).content(ctx, rect) - ctx.tokens.md;
    (widest as f32 * advance - room).max(0.0)
}

/// The row and the column a point lands on, counted from the first row.
pub fn code_at(
    tokens: &Tokens,
    chars: Chars,
    rect: Rect,
    point: (f32, f32),
) -> Option<(usize, usize)> {
    if !rect.contains(point.0, point.1) {
        return None;
    }
    let row = ((point.1 - rect.y + chars.scroll) / tokens.line)
        .floor()
        .max(0.0);
    let column = ((point.0 - chars.left) / chars.advance.max(1.0))
        .round()
        .max(0.0);
    Some((row as usize, column as usize))
}
