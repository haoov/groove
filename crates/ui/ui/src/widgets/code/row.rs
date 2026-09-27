//! One row drawn: its ground, its marks, its numbers and its text.

mod note;

use self::note::{acts, note};

use groove_gfx::{Align, Color, Edges, Rect, TextStyle};

use super::gutter::Block;
use super::{Line, head_mark};
use crate::base::ctx::Ctx;
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::{ruled, square};
use crate::text::{Label, row};

pub(super) fn draw(ctx: &mut Ctx, line: Rect, code: &Line<'_>, gutter: Block) {
    if code.band {
        return band(ctx, line, code.text);
    }
    if code.head {
        return head(ctx, line, code);
    }
    if code.banner {
        return banner(ctx, line, code.text);
    }
    if let Some(said) = code.said {
        return note(ctx, line, code.text, said, gutter);
    }
    if let Some(acting) = code.acting.as_ref() {
        return acts(ctx, line, acting, gutter);
    }
    coded(ctx, line, code, gutter);
}

/// A line of code: its grounds, its marks, its numbers and its text.
fn coded(ctx: &mut Ctx, line: Rect, code: &Line<'_>, gutter: Block) {
    if code.noted {
        ctx.quad(line, ctx.styles.noted());
    }
    if let Some(ground) = code.ground {
        ctx.quad(line, ground);
    }
    if let Some(mark) = code.mark {
        let edge = Rect {
            w: ctx.tokens.hairline * 2.0,
            ..line
        };
        ctx.quad(edge, mark);
    }
    numbers(ctx, line, code, gutter);
    let at = gutter.content(ctx, line);
    let rect = line.pad(Edges::across(at - line.x, 0.0));
    if let Some(color) = code.word {
        for (from, to) in code.words {
            shade(ctx, rect, code.text, (*from, *to), color);
        }
    }
    for (from, to) in code.found {
        marked(ctx, rect, code.text, (*from, *to), false);
    }
    if let Some(at) = code.standing {
        marked(ctx, rect, code.text, at, true);
    }
    if let Some(held) = code.held {
        holding(ctx, rect, code.text, held);
    }
    text(ctx, rect, code);
    if let Some(column) = code.caret {
        caret(ctx, rect, code.text, column);
        here(ctx, line);
    }
}

/// The row's own line numbers, one to a gutter cell.
fn numbers(ctx: &mut Ctx, line: Rect, code: &Line<'_>, gutter: Block) {
    let (sm, style) = (ctx.tokens.sm, ctx.styles.code(Role::Ghost));
    let mut room = line.pad(Edges::across(sm, 0.0));
    for text in code.gutters {
        let cell = room.take_left(gutter.width);
        room.take_left(sm);
        let width = ctx.measure(text, &style);
        row(ctx, cell, cell.w - width, text, style);
    }
}

/// Where the caret is: a rule above the row and one below it, across the surface.
fn here(ctx: &mut Ctx, line: Rect) {
    let color = ctx.styles.here();
    ruled(ctx, line, color);
}

/// A caret's band under the text, run past the line's end when the selection carries on.
fn holding(ctx: &mut Ctx, rect: Rect, text: &str, held: (usize, usize, bool)) {
    let (from, to, through) = held;
    let style = ctx.styles.code(Role::Text);
    let start = upto(ctx, text, from, &style);
    let mut end = upto(ctx, text, to, &style);
    if through {
        end += ctx.measure("M", &style);
    }
    let color = ctx.styles.held();
    ctx.quad(
        Rect::new(rect.x + start, rect.y, (end - start).max(1.0), rect.h),
        color,
    );
}

/// What a search found, under the text; the one it stands on reads as a selection.
fn marked(ctx: &mut Ctx, rect: Rect, text: &str, at: (usize, usize), standing: bool) {
    let color = match standing {
        true => ctx.styles.held(),
        false => ctx.styles.found(),
    };
    shade(ctx, rect, text, at, color);
}

/// A band of `color` under the columns `at` covers.
fn shade(ctx: &mut Ctx, rect: Rect, text: &str, at: (usize, usize), color: Color) {
    let style = ctx.styles.code(Role::Text);
    let start = upto(ctx, text, at.0, &style);
    let end = upto(ctx, text, at.1, &style);
    ctx.quad(
        Rect::new(rect.x + start, rect.y, (end - start).max(1.0), rect.h),
        color,
    );
}

/// How wide the first `column` characters are.
fn upto(ctx: &mut Ctx, text: &str, column: usize, style: &TextStyle) -> f32 {
    let before: String = text.chars().take(column).collect();
    ctx.measure(&before, style)
}

/// The caret: a bar at the column, placed through the text before it.
fn caret(ctx: &mut Ctx, rect: Rect, text: &str, column: usize) {
    let style = ctx.styles.code(Role::Text);
    let at = rect.x + upto(ctx, text, column, &style);
    let (width, color) = (ctx.tokens.hairline * 2.0, ctx.styles.caret());
    ctx.quad(Rect::new(at, rect.y, width, rect.h), color);
}

/// A row across the width: its own ground, its text in the middle.
fn banner(ctx: &mut Ctx, line: Rect, text: &str) {
    ctx.quad(line, ctx.styles.band());
    let label = Label::new(text, ctx.styles.code(Role::Ghost));
    let width = label.width(ctx);
    label.draw(
        ctx,
        line.align((width, line.h), Align::Center, Align::Start),
    );
}

/// A row naming a directory: its own ground, its path faint at the margin.
fn band(ctx: &mut Ctx, line: Rect, text: &str) {
    let (ground, style) = (ctx.styles.band(), ctx.styles.small(Role::Faint));
    ctx.quad(line, ground);
    row(ctx, line, ctx.tokens.md, text, style);
}

/// A row naming a file: its own ground, a caret for its rows, its name.
fn head(ctx: &mut Ctx, line: Rect, code: &Line<'_>) {
    let role = match code.read {
        true => Role::Faint,
        false => Role::Text,
    };
    let (ground, style) = (ctx.styles.raised(), ctx.styles.label(role));
    ctx.quad(line, ground);
    let (xs, size) = (ctx.tokens.xs, ctx.tokens.icon);
    let mut room = line.pad(Edges::across(xs, 0.0));
    let caret = square(room.take_left(size), size);
    room.take_left(xs);
    let turn = match code.folded {
        true => Mark::RIGHTWARDS,
        false => 0,
    };
    ctx.icon(caret, Mark::Down, turn, ctx.styles.color(Role::Faint));
    row(ctx, room, 0.0, code.text, style);
    box_(ctx, head_mark(ctx, line), code.read);
}

/// The box that says whether a file is read, ticked once it is.
fn box_(ctx: &mut Ctx, rect: Rect, read: bool) {
    let square = rect.pad(Edges::all(ctx.tokens.xs / 2.0));
    let role = match read {
        true => Role::Text,
        false => Role::Faint,
    };
    ctx.border(square, ctx.styles.color(role));
    if read {
        ctx.icon(rect, Mark::Read, 0, ctx.styles.color(role));
    }
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
