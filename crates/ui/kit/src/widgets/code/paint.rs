//! How a row of code is painted: its grounds, its marks, its numbers and its text.

use groove_gfx::{Align, Color, Edges, Rect, TextStyle};

use super::gutter::tiled;
use super::{Block, Code};
use crate::base::ctx::{App, Ctx};
use crate::base::ground::Ground;
use crate::base::style::Role;
use crate::shape::ruled;
use crate::text::{Label, row};
use crate::widgets::{caret_bar, text_band};

/// A line of code: its grounds, its marks, its numbers and its text, `across` to the left.
pub(super) fn coded<A: App>(
    ctx: &mut Ctx<'_, A>,
    line: Rect,
    code: &Code<'_, A::Target>,
    gutter: Block,
    across: f32,
) {
    if code.empty {
        let (color, gap) = (ctx.styles.hatch(), ctx.tokens.hatch);
        return ctx.hatched(line, color, gap);
    }
    if code.noted {
        crate::shape::ground(ctx, line, Ground::Noted);
    }
    if let Some(ground) = code.ground {
        ctx.quad(line, ground);
    }
    if let Some((ground, _)) = code.gutter {
        let end = gutter.content(ctx, line) - ctx.tokens.md;
        ctx.quad(
            Rect {
                w: end - line.x,
                ..line
            },
            ground,
        );
    }
    if let Some(mark) = code.mark {
        let edge = Rect {
            w: ctx.tokens.hairline * 2.0,
            ..line
        };
        ctx.quad(edge, mark);
    }
    numbers(ctx, line, code, gutter);
    super::outline::caret(ctx, line, code, gutter);
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
    super::outline::guides(ctx, rect, code);
    super::outline::occurs(ctx, rect, code);
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
    super::outline::hidden(ctx, rect, code);
    super::faults::faulted(ctx, rect, code);
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
    let mut style = ctx.styles.source(Role::Text);
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
    let (sm, mut style) = (ctx.tokens.sm, ctx.styles.code(Role::Ghost));
    if let Some((_, color)) = code.gutter {
        style.color = color;
    }
    if let Some(color) = super::faults::worst(ctx, code) {
        style.color = color;
    }
    if let Some((tile, opens)) = code.tile.as_ref() {
        tiled(ctx, gutter.marked(ctx, line), *tile, opens.clone());
    }
    let mut room = line.pad(Edges::across(sm + gutter.mark, 0.0));
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
    let style = ctx.styles.source(Role::Text);
    let start = upto(ctx, text, from, &style);
    let mut end = upto(ctx, text, to, &style);
    if through {
        end += ctx.advance;
    }
    let color = ctx.styles.held();
    text_band(ctx, rect, (start, end), color);
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
    let style = ctx.styles.source(Role::Text);
    let start = upto(ctx, text, at.0, &style);
    let end = upto(ctx, text, at.1, &style);
    text_band(ctx, rect, (start, end), color);
}

/// How wide the first `column` characters are; ASCII by the code font's advance, unshaped.
pub(super) fn upto<A: App>(
    ctx: &mut Ctx<'_, A>,
    text: &str,
    column: usize,
    style: &TextStyle,
) -> f32 {
    let before: String = text.chars().take(column).collect();
    match before.is_ascii() {
        true => before.len() as f32 * ctx.advance,
        false => ctx.measure(&before, style),
    }
}

/// The caret: a bar at the column, placed through the text before it.
fn caret<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, text: &str, column: usize) {
    let style = ctx.styles.source(Role::Text);
    let at = rect.x + upto(ctx, text, column, &style);
    let color = ctx.styles.caret();
    caret_bar(ctx, at, rect, color);
}

/// A row across the width: a rule above and below, its text in the middle.
pub(super) fn banner<A: App>(ctx: &mut Ctx<'_, A>, line: Rect, text: &str) {
    let rule = ctx.styles.line();
    ruled(ctx, line, rule);
    let label = Label::new(text, ctx.styles.code(Role::Ghost));
    let width = label.width(ctx);
    label.draw(
        ctx,
        line.align((width, line.h), Align::Center, Align::Start),
    );
}

/// The line's text, in the pieces its spans cut it into.
fn text<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, code: &Code<'_, A::Target>) {
    let plain = ctx.styles.source(Role::Text);
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
        style.color = match code.gutter {
            Some(_) => ctx.styles.changed_syntax(span.capture),
            None => ctx.styles.syntax(span.capture),
        };
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
