use std::ops::Range;

use groove_gfx::{CellSize, Color, Rect, TextStyle};
use groove_types::Highlight;

use crate::ctx::Ctx;
use crate::style::Role;
use crate::tokens::Tokens;
use crate::widget::row;

/// One row of code: what its gutters say, its text, and the colour over it.
pub struct Line<'a> {
    pub gutters: &'a [&'a str],
    pub text: &'a str,
    pub spans: &'a [Highlight],
    pub ground: Option<Color>,
    /// A bar at the row's left edge, for what a gutter number cannot say.
    pub mark: Option<Color>,
    /// A row across the whole width with no gutters: a gap, a note.
    pub banner: bool,
}

impl<'a> Line<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            gutters: &[],
            text,
            spans: &[],
            ground: None,
            mark: None,
            banner: false,
        }
    }

    /// A row that spans the width, centred, on its own ground.
    pub fn banner(text: &'a str) -> Self {
        Self {
            banner: true,
            ..Self::new(text)
        }
    }

    pub fn gutters(mut self, gutters: &'a [&'a str]) -> Self {
        self.gutters = gutters;
        self
    }

    pub fn spans(mut self, spans: &'a [Highlight]) -> Self {
        self.spans = spans;
        self
    }

    pub fn ground(mut self, ground: Color) -> Self {
        self.ground = ground.into();
        self
    }

    pub fn mark(mut self, mark: Option<Color>) -> Self {
        self.mark = mark;
        self
    }
}

/// The rows a surface draws: the window it built, and the gutter they share.
pub struct Rows<'a> {
    pub lines: &'a [Line<'a>],
    /// Where the first line sits among all the rows.
    pub first: usize,
    pub gutters: Gutters,
}

/// The gutter a surface asks for: how many number columns, and their longest number.
#[derive(Debug, Clone, Copy)]
pub struct Gutters {
    pub cells: usize,
    pub digits: usize,
}

/// The gutter block, measured once for the surface.
#[derive(Debug, Clone, Copy)]
struct Block {
    cells: usize,
    width: f32,
}

impl Block {
    fn of(ctx: &mut Ctx, gutters: Gutters) -> Self {
        let numbers = ctx.styles.code(Role::Ghost);
        let widest = "0".repeat(gutters.digits);
        Self {
            cells: gutters.cells,
            width: ctx.measure(&widest, &numbers),
        }
    }

    /// Where a line's text starts, past every gutter and the hairline.
    fn content(&self, ctx: &Ctx, rect: Rect) -> f32 {
        let small = ctx.tokens.sm;
        let block = (self.width + small) * self.cells as f32;
        rect.x + small + block + ctx.tokens.md
    }
}

/// The rows `rect` has room for at `scroll`, among `total`.
pub fn visible(ctx: &Ctx, rect: Rect, total: usize, scroll: f32) -> Range<usize> {
    let height = ctx.tokens.line;
    let first = ((scroll / height).floor().max(0.0) as usize).min(total);
    let shown = (rect.h / height).ceil() as usize + 1;
    first..(first + shown).min(total)
}

/// Rows of code from the top of `rect`, scrolled by `scroll`, clipped to it.
pub fn code(ctx: &mut Ctx, rect: Rect, rows: Rows<'_>, scroll: f32) {
    let height = ctx.tokens.line;
    let block = Block::of(ctx, rows.gutters);
    ctx.clipped(rect, |ctx| {
        let mut y = rect.y - scroll + rows.first as f32 * height;
        for line in rows.lines {
            draw(ctx, Rect::new(rect.x, y, rect.w, height), line, block);
            y += height;
        }
    });
    rule(ctx, rect, block);
}

/// How tall the rows stand together.
pub fn height(ctx: &Ctx, lines: usize) -> f32 {
    ctx.tokens.line * lines as f32
}

/// The row and the column a point lands on, counted from the first row.
pub fn code_at(
    tokens: &Tokens,
    cell: CellSize,
    rect: Rect,
    scroll: f32,
    point: (f32, f32),
) -> Option<(usize, usize)> {
    if !rect.contains(point.0, point.1) {
        return None;
    }
    let row = ((point.1 - rect.y + scroll) / tokens.line).floor().max(0.0);
    let text = rect.x + tokens.sm;
    let column = ((point.0 - text) / cell.width.max(1.0)).floor().max(0.0);
    Some((row as usize, column as usize))
}

/// The hairline the gutters end at, down the whole surface.
fn rule(ctx: &mut Ctx, rect: Rect, gutter: Block) {
    if gutter.cells == 0 {
        return;
    }
    let (color, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    let at = gutter.content(ctx, rect) - ctx.tokens.md;
    ctx.quad(Rect::new(at, rect.y, thickness, rect.h), color);
}

fn draw(ctx: &mut Ctx, line: Rect, code: &Line<'_>, gutter: Block) {
    if code.banner {
        return banner(ctx, line, code.text);
    }
    if let Some(ground) = code.ground {
        ctx.quad(line, ground);
    }
    if let Some(mark) = code.mark {
        let width = ctx.tokens.hairline * 2.0;
        ctx.quad(Rect::new(line.x, line.y, width, line.h), mark);
    }
    let numbers = ctx.styles.code(Role::Ghost);
    let mut at = line.x + ctx.tokens.sm;
    for text in code.gutters {
        let width = ctx.measure(text, &numbers);
        let cell = Rect::new(at + gutter.width - width, line.y, width, line.h);
        row(ctx, cell, 0.0, text, numbers);
        at += gutter.width + ctx.tokens.sm;
    }
    let at = gutter.content(ctx, line);
    text(ctx, Rect::new(at, line.y, line.right() - at, line.h), code);
}

/// A row across the width: its own ground, its text in the middle.
fn banner(ctx: &mut Ctx, line: Rect, text: &str) {
    let (panel, style) = (ctx.styles.panel(), ctx.styles.code(Role::Ghost));
    ctx.quad(line, panel);
    let width = ctx.measure(text, &style);
    let at = line.x + (line.w - width) / 2.0;
    row(ctx, Rect::new(at, line.y, width, line.h), 0.0, text, style);
}

/// The line's text, in the pieces its spans cut it into.
fn text(ctx: &mut Ctx, rect: Rect, code: &Line<'_>) {
    let plain = ctx.styles.code(Role::Text);
    if code.spans.is_empty() {
        return row(ctx, rect, 0.0, code.text, plain);
    }
    let mut at = rect.x;
    let mut cut = 0;
    for span in code.spans {
        let Some(before) = code.text.get(cut..span.range.start) else {
            break;
        };
        at = piece(ctx, rect, at, before, plain);
        let Some(inside) = code.text.get(span.range.clone()) else {
            break;
        };
        let mut style = plain;
        style.color = ctx.styles.syntax(span.capture);
        at = piece(ctx, rect, at, inside, style);
        cut = span.range.end;
    }
    if let Some(rest) = code.text.get(cut..) {
        piece(ctx, rect, at, rest, plain);
    }
}

/// One piece of a line at `x`. Returns where the next one starts.
fn piece(ctx: &mut Ctx, rect: Rect, x: f32, text: &str, style: TextStyle) -> f32 {
    if text.is_empty() {
        return x;
    }
    let width = ctx.measure(text, &style);
    row(ctx, Rect::new(x, rect.y, width, rect.h), 0.0, text, style);
    x + width
}
