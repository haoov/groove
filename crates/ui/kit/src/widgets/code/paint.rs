//! How a row of code is painted: its grounds, its marks, its numbers and its text.

use groove_gfx::{Align, Color, Edges, Rect, TextStyle};

use super::{Block, Code, head_mark};
use crate::base::ctx::{App, Ctx};
use crate::base::ground::Ground;
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::ruled;
use crate::text::{Label, row};

/// A line of code: its grounds, its marks, its numbers and its text, `across` to the left.
pub(super) fn coded<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    code: &Code<'_, A::Target>,
    gutter: Block,
    across: f32,
) {
    if code.noted {
        crate::shape::ground(ctx, line, Ground::Noted);
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
    let room = line.pad(Edges::across(at - line.x, 0.0));
    let rect = Rect {
        x: room.x - across,
        w: room.w + across,
        ..room
    };
    ctx.clipped(room, |ctx| over(ctx, (rect, room), code));
    if code.caret.is_some() {
        here(ctx, line);
    }
}

/// What lies over a line's ground, inside its text's room: shades, text, caret, blame.
fn over<A: App>(ctx: &mut Ctx<'_, A>, (rect, room): (Rect, Rect), code: &Code<'_, A::Target>) {
    if let Some(color) = code.word {
        for (from, to) in code.words {
            shade(ctx, rect, code.text, (*from, *to), color);
        }
    }
    for (from, to) in code.found {
        marked(ctx, rect, code.text, (*from, *to), false);
    }
    if let Some(held) = code.held {
        holding(ctx, rect, code.text, held);
    }
    if let Some(at) = code.standing {
        marked(ctx, rect, code.text, at, true);
    }
    text(ctx, rect, code);
    if let Some(column) = code.caret {
        caret(ctx, rect, code.text, column);
    }
    if let Some((said, sha)) = code.blame.as_ref() {
        blamed(ctx, (rect, room), code.text, (*said, sha.clone()));
    }
}

/// Who last changed the line, faint after its end; a click on it opens the commit.
fn blamed<A: App>(
    ctx: &mut Ctx<'_, A>,
    (rect, room): (Rect, Rect),
    text: &str,
    (said, sha): (&str, Option<A::Target>),
) {
    let mut style = ctx.styles.code(Role::Text);
    let end = ctx.measure(text, &style);
    style.color = ctx.styles.syntax(groove_types::Capture::Comment);
    let x = rect.x + end + ctx.tokens.lg;
    let width = ctx.measure(said, &style);
    let at = Rect::new(x, rect.y, width, rect.h);
    row(ctx, at, 0.0, said, style);
    let Some(sha) = sha else {
        return;
    };
    // The short sha is the last word of what the blame says.
    let before = said.rfind(' ').map_or(0, |space| space + 1);
    let from = (x + ctx.measure(&said[..before], &style)).max(room.x);
    let shown = Rect::new(
        from,
        rect.y,
        (at.right().min(room.right()) - from).max(0.0),
        rect.h,
    );
    if shown.w > 0.0 {
        ctx.hit(shown, sha);
    }
}

/// The row's own line numbers, one to a gutter cell.
fn numbers<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, code: &Code<'_, A::Target>, gutter: Block) {
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
fn here<A: App>(ctx: &mut Ctx<'_, A>, line: Rect) {
    let color = ctx.styles.here();
    ruled(ctx, line, color);
}

/// A caret's band under the text, run past the line's end when the selection carries on.
fn holding<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, text: &str, held: (usize, usize, bool)) {
    let (from, to, through) = held;
    let style = ctx.styles.code(Role::Text);
    let start = upto(ctx, text, from, &style);
    let mut end = upto(ctx, text, to, &style);
    if through {
        end += ctx.advance;
    }
    let color = ctx.styles.held();
    ctx.quad(
        Rect::new(rect.x + start, rect.y, (end - start).max(1.0), rect.h),
        color,
    );
}

/// What a search found, under the text; the one it stands on in peach.
fn marked<A: App>(
    ctx: &mut Ctx<'_, A>,
    rect: Rect,
    text: &str,
    at: (usize, usize),
    standing: bool,
) {
    let color = match standing {
        true => ctx.styles.standing(),
        false => ctx.styles.found(),
    };
    shade(ctx, rect, text, at, color);
}

/// A band of `color` under the columns `at` covers.
fn shade<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, text: &str, at: (usize, usize), color: Color) {
    let style = ctx.styles.code(Role::Text);
    let start = upto(ctx, text, at.0, &style);
    let end = upto(ctx, text, at.1, &style);
    ctx.quad(
        Rect::new(rect.x + start, rect.y, (end - start).max(1.0), rect.h),
        color,
    );
}

/// How wide the first `column` characters are.
fn upto<A: App>(ctx: &mut Ctx<'_, A>, text: &str, column: usize, style: &TextStyle) -> f32 {
    let before: String = text.chars().take(column).collect();
    ctx.measure(&before, style)
}

/// The caret: a bar at the column, placed through the text before it.
fn caret<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, text: &str, column: usize) {
    let style = ctx.styles.code(Role::Text);
    let at = rect.x + upto(ctx, text, column, &style);
    let (width, color) = (ctx.tokens.hairline * 2.0, ctx.styles.caret());
    ctx.quad(Rect::new(at, rect.y, width, rect.h), color);
}

/// A row across the width: its own ground, its text in the middle.
pub(super) fn banner<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, text: &str) {
    crate::shape::ground(ctx, line, Ground::Band);
    let label = Label::new(text, ctx.styles.code(Role::Ghost));
    let width = label.width(ctx);
    label.draw(
        ctx,
        line.align((width, line.h), Align::Center, Align::Start),
    );
}

/// A row naming a file: its own ground, a caret for its rows, its name.
pub(super) fn head<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, code: &Code<'_, A::Target>) {
    let role = match code.read {
        true => Role::Faint,
        false => Role::Text,
    };
    let (ground, style) = (ctx.styles.raised(), ctx.styles.label(role));
    ctx.quad(line, ground);
    let (xs, size) = (ctx.tokens.xs, ctx.tokens.icon);
    let mut room = line.pad(Edges::across(xs, 0.0));
    crate::widgets::caret(ctx, &mut room, (!code.folded, size), Role::Faint);
    row(ctx, room, 0.0, code.text, style);
    box_(ctx, head_mark(ctx, line), code.read);
}

/// The box that says whether a file is read, ticked once it is.
fn box_<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, read: bool) {
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
fn text<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, code: &Code<'_, A::Target>) {
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
fn piece<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, x: f32, text: &str, style: TextStyle) -> f32 {
    if text.is_empty() {
        return x;
    }
    let width = ctx.measure(text, &style);
    row(ctx, Rect::new(x, rect.y, width, rect.h), 0.0, text, style);
    x + width
}
