//! The bars themselves: the row each task keeps, and the shape its dates give it.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::{Day, Span, Task, Timestamp};

use super::{DAYS, first};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::{elide, row};

/// One bar a task, in the row the whole plan gives it, where the horizon puts it.
pub(super) fn draw(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let step = body.w / DAYS as f32;
    let top = body.y + ctx.tokens.line;
    for (task, span, at) in placed(app, ui) {
        if !inside(span, ui) {
            continue;
        }
        let (from, to) = reach(span, ui);
        let x = body.x + step * from as f32;
        let wide = (step * (to - from) as f32).max(step / 2.0);
        let line = Rect::new(x, top + ctx.tokens.row * at as f32, wide, ctx.tokens.row);
        match span {
            Span::Point(_) => point(ctx, line, app, task),
            _ => one(ctx, line, app, task, matches!(span, Span::Open { .. })),
        }
        ctx.hit(line, Target::Bar(task.short_id.clone()));
    }
}

/// Every task with dates, in the row it keeps wherever the horizon stands: the first
/// row no bar of it already reaches into.
fn placed<'a>(app: &'a AppState, ui: &Ui) -> Vec<(&'a Task, Span, usize)> {
    let mut spans = dated(app, ui);
    spans.sort_by(|one, other| {
        let (a, b) = (from_today(one.1), from_today(other.1));
        a.0.cmp(&b.0)
            .then_with(|| one.0.short_id.cmp(&other.0.short_id))
    });
    let mut ends: Vec<i64> = Vec::new();
    spans
        .into_iter()
        .map(|(task, span)| {
            let (from, to) = from_today(span);
            let at = match ends.iter().position(|end| *end < from) {
                Some(at) => at,
                None => {
                    ends.push(i64::MIN);
                    ends.len() - 1
                }
            };
            ends[at] = to;
            (task, span, at)
        })
        .collect()
}

/// A span's days, counted from today, which no horizon moves.
fn from_today(span: Span) -> (i64, i64) {
    days_from(Timestamp::now().day(), span)
}

/// One bar: its ground, its edge, and its title inside it.
fn one(ctx: &mut Ctx, line: Rect, app: &AppState, task: &Task, open: bool) {
    let asking = app.task.asks(&task.external_id);
    let box_ = Rect::new(
        line.x,
        line.y + ctx.tokens.xs,
        line.w,
        line.h - ctx.tokens.sm,
    );
    let role = match asking {
        true => Role::Attention,
        false => Role::Text,
    };
    if !open {
        ctx.quad(box_, ctx.styles.line());
    }
    if asking || open {
        ctx.border(box_, ctx.styles.color(role));
    }
    let style = ctx.styles.small(role);
    let text = elide(ctx, &task.title, &style, box_.w - ctx.tokens.sm * 2.0);
    row(ctx, box_, ctx.tokens.sm, &text, style);
}

/// A task with a due date and nothing else: a mark, and its title beside it.
fn point(ctx: &mut Ctx, line: Rect, app: &AppState, task: &Task) {
    let asking = app.task.asks(&task.external_id);
    let role = match asking {
        true => Role::Attention,
        false => Role::Muted,
    };
    let size = ctx.tokens.sm;
    let dot = Rect::new(line.x, line.y + (line.h - size) / 2.0, size, size);
    ctx.quad(dot, ctx.styles.color(role));
    let style = ctx.styles.small(role);
    let room = ctx.tokens.aside_far;
    let text = elide(ctx, &task.title, &style, room);
    let at = dot.right() + ctx.tokens.xs;
    row(ctx, Rect::new(at, line.y, room, line.h), 0.0, &text, style);
}

/// Every task the filter lets through whose dates fall in the horizon.
pub(super) fn drawn<'a>(app: &'a AppState, ui: &Ui) -> Vec<(&'a Task, Span)> {
    dated(app, ui)
        .into_iter()
        .filter(|(_, span)| inside(*span, ui))
        .collect()
}

/// Every task the filter lets through that carries dates at all.
fn dated<'a>(app: &'a AppState, ui: &Ui) -> Vec<(&'a Task, Span)> {
    let query = ui.board.query();
    app.task
        .tasks
        .iter()
        .filter(|task| query.lets_task(task))
        .filter_map(|task| task.dates.span().map(|span| (task, span)))
        .collect()
}

/// Where a span starts and ends, in days from the band's first.
fn reach(span: Span, ui: &Ui) -> (i64, i64) {
    let (from, to) = days(span, ui);
    (from.clamp(0, DAYS), to.clamp(from.clamp(0, DAYS), DAYS))
}

/// Whether any of a span's days stands in the horizon.
fn inside(span: Span, ui: &Ui) -> bool {
    let (from, to) = days(span, ui);
    to >= 0 && from < DAYS
}

/// A span's own days, counted from the band's first.
fn days(span: Span, ui: &Ui) -> (i64, i64) {
    days_from(first(ui), span)
}

/// Where a span starts and ends, in days from `origin`. An open one ends today.
fn days_from(origin: Day, span: Span) -> (i64, i64) {
    let today = origin.days_until(Timestamp::now().day());
    match span {
        Span::Bar { from, to } => (origin.days_until(from), origin.days_until(to)),
        Span::Open { from } => (origin.days_until(from), today),
        Span::Point(due) => (origin.days_until(due), origin.days_until(due)),
    }
}
