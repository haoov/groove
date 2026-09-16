use groove_gfx::{CellSize, Color, Rect};
use groove_types::Highlight;

use crate::ctx::Ctx;
use crate::style::Role;
use crate::tokens::Tokens;
use crate::widget::row;

/// One row of code: what its gutters say, its text, and the colour over it.
pub struct Line<'a> {
    pub gutters: [&'a str; 2],
    pub text: &'a str,
    pub spans: &'a [Highlight],
    pub ground: Option<Color>,
    /// A row across the whole width with no gutters: a gap, a note.
    pub banner: bool,
}

impl<'a> Line<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            gutters: ["", ""],
            text,
            spans: &[],
            ground: None,
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

    pub fn gutters(mut self, gutters: [&'a str; 2]) -> Self {
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
}

/// Rows of code from the top of `rect`, scrolled by `scroll`, clipped to it. The
/// gutters take the width of the widest of them.
pub fn code(ctx: &mut Ctx, rect: Rect, lines: &[Line<'_>], scroll: f32) {
    let height = ctx.tokens.line;
    let gutter = gutter_width(ctx, lines);
    ctx.clipped(rect, |ctx| {
        let first = (scroll / height).floor().max(0.0);
        let shown = (rect.h / height).ceil() as usize + 1;
        let skipped = first as usize;
        let mut y = rect.y - scroll + first * height;
        for line in lines.iter().skip(skipped).take(shown) {
            draw(ctx, Rect::new(rect.x, y, rect.w, height), line, gutter);
            y += height;
        }
    });
    rule(ctx, rect, gutter);
}

/// The hairline the gutters end at, down the whole surface.
fn rule(ctx: &mut Ctx, rect: Rect, gutter: f32) {
    let (color, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    let at = content(ctx, rect, gutter) - ctx.tokens.md;
    ctx.quad(Rect::new(at, rect.y, thickness, rect.h), color);
}

/// Where a line's text starts, past both gutters and the hairline.
fn content(ctx: &Ctx, rect: Rect, gutter: f32) -> f32 {
    let (small, wide) = (ctx.tokens.sm, ctx.tokens.md);
    rect.x + small + (gutter + small) * 2.0 + wide
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

fn draw(ctx: &mut Ctx, line: Rect, code: &Line<'_>, gutter: f32) {
    if code.banner {
        return banner(ctx, line, code.text);
    }
    if let Some(ground) = code.ground {
        ctx.quad(line, ground);
    }
    let numbers = ctx.styles.code(Role::Ghost);
    let mut at = line.x + ctx.tokens.sm;
    for text in code.gutters {
        let width = ctx.measure(text, &numbers);
        let cell = Rect::new(at + gutter - width, line.y, width, line.h);
        row(ctx, cell, 0.0, text, numbers);
        at += gutter + ctx.tokens.sm;
    }
    let at = content(ctx, line, gutter);
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
fn piece(ctx: &mut Ctx, rect: Rect, x: f32, text: &str, style: groove_gfx::TextStyle) -> f32 {
    if text.is_empty() {
        return x;
    }
    let width = ctx.measure(text, &style);
    row(ctx, Rect::new(x, rect.y, width, rect.h), 0.0, text, style);
    x + width
}

fn gutter_width(ctx: &mut Ctx, lines: &[Line<'_>]) -> f32 {
    let numbers = ctx.styles.code(Role::Ghost);
    lines
        .iter()
        .flat_map(|line| line.gutters)
        .map(|text| ctx.measure(text, &numbers))
        .fold(0.0, f32::max)
}
