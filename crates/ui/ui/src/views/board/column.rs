//! One column of the board: its heading, then a row per item.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;
use groove_types::{SessionKind, Task};

use super::List;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{after_mark, elide, hairline, icon, leading, row};

pub(super) fn draw(ctx: &mut Ctx, area: Rect, app: &AppState, ui: &Ui, list: List) {
    let live: Vec<&Open> = app.session.open.iter().collect();
    let next: Vec<&Task> = planned(app);
    let count = match list {
        List::Live => live.len(),
        List::Next => next.len(),
        List::Review => 0,
    };
    let head = Rect::new(area.x, area.y, area.w, ctx.tokens.header);
    heading(ctx, head, list, count);
    edge(ctx, area);

    let body = Rect::new(area.x, head.bottom(), area.w, area.h - head.h);
    match list {
        List::Live if live.is_empty() => {
            said(ctx, body, "nothing open. Ctrl+Shift+N starts an explorer")
        }
        List::Live => rows(ctx, body, ui, list, live.len(), |ctx, line, at| {
            session(ctx, line, ui, live[at])
        }),
        List::Next => rows(ctx, body, ui, list, next.len(), |ctx, line, at| {
            task(ctx, line, next[at])
        }),
        List::Review => said(ctx, body, "reviews arrive with the MRs"),
    }
}

/// The tasks with no session of their own, in the order the source answered.
fn planned(app: &AppState) -> Vec<&Task> {
    app.task
        .tasks
        .iter()
        .filter(|task| !app.session.open.iter().any(|open| holds(open, task)))
        .collect()
}

/// Whether this session is the one working that task.
fn holds(open: &Open, task: &Task) -> bool {
    match &open.session.kind {
        SessionKind::Task { external_id } => *external_id == task.external_id,
        _ => false,
    }
}

fn heading(ctx: &mut Ctx, line: Rect, list: List, count: usize) {
    ctx.quad(line, ctx.styles.panel());
    hairline(ctx, line, ctx.styles.line());
    let label = match count {
        0 => list.name().to_string(),
        n => format!("{} · {n}", list.name()),
    };
    row(
        ctx,
        line,
        ctx.tokens.md,
        &label,
        ctx.styles.heading(Role::Faint),
    );
}

/// The rule that separates this column from the one before it.
fn edge(ctx: &mut Ctx, area: Rect) {
    let (rule, thickness) = (ctx.styles.line(), ctx.tokens.hairline);
    ctx.quad(Rect::new(area.x, area.y, thickness, area.h), rule);
}

/// The rows a column has room for, scrolled and clipped to it.
fn rows(
    ctx: &mut Ctx,
    body: Rect,
    ui: &Ui,
    list: List,
    count: usize,
    mut one: impl FnMut(&mut Ctx, Rect, usize),
) {
    let height = ctx.tokens.row;
    let extent = (height * count as f32 - body.h).max(0.0);
    ctx.scrolls(Scroller::Column(list as u8), extent);
    let scroll = ui.board.scroll(list).min(extent);
    ctx.clipped(body, |ctx| {
        for at in 0..count {
            let y = body.y - scroll + height * at as f32;
            if y + height < body.y || y > body.bottom() {
                continue;
            }
            let line = Rect::new(body.x, y, body.w, height);
            one(ctx, line, at);
            hairline(ctx, line, ctx.styles.line());
        }
    });
}

/// One open session: its kind, its title, and how many repos it holds.
fn session(ctx: &mut Ctx, line: Rect, ui: &Ui, open: &Open) {
    let target = Target::Session(open.session.id.clone());
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(line, ctx.styles.hover());
    }
    ctx.hit(line, target);
    let box_ = leading(ctx, line, ctx.tokens.md);
    icon(ctx, box_, Mark::of_kind(&open.session.kind), Role::Faint);
    let repos = match open.repos.len() {
        0 => String::new(),
        n => format!("{n} repos"),
    };
    let at = aside(ctx, line, &repos);
    named(ctx, line, &open.session.title, at);
}

/// One task waiting: its title, and what it is worth.
fn task(ctx: &mut Ctx, line: Rect, task: &Task) {
    let box_ = leading(ctx, line, ctx.tokens.md);
    icon(ctx, box_, Mark::Task, Role::Ghost);
    let at = aside(ctx, line, &worth(task));
    named(ctx, line, &task.title, at);
}

/// The priority and the estimate, as the row's right-hand text.
fn worth(task: &Task) -> String {
    let priority = task.priority.map(|one| one.label().to_string());
    let estimate = task.estimate.map(|hours| format!("{hours}h"));
    [priority, estimate]
        .into_iter()
        .flatten()
        .collect::<Vec<String>>()
        .join(" · ")
}

/// A row's right-hand text. Returns where it starts.
fn aside(ctx: &mut Ctx, line: Rect, text: &str) -> f32 {
    if text.is_empty() {
        return line.right() - ctx.tokens.md;
    }
    let style = ctx.styles.small(Role::Faint);
    let width = ctx.measure(text, &style);
    let at = line.right() - ctx.tokens.md - width;
    row(ctx, Rect::new(at, line.y, width, line.h), 0.0, text, style);
    at
}

/// A row's title, cut where its right-hand text starts.
fn named(ctx: &mut Ctx, line: Rect, title: &str, until: f32) {
    let style = ctx.styles.label(Role::Text);
    let indent = after_mark(ctx, ctx.tokens.md);
    let room = (until - line.x - indent - ctx.tokens.sm).max(0.0);
    let text = elide(ctx, title, &style, room);
    row(ctx, line, indent, &text, style);
}

/// What a column says while it holds nothing.
fn said(ctx: &mut Ctx, body: Rect, text: &str) {
    let style = ctx.styles.small(Role::Faint);
    let line = Rect::new(body.x, body.y, body.w, ctx.tokens.row);
    row(ctx, line, ctx.tokens.md, text, style);
}
