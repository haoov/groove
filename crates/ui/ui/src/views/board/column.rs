//! One column of the board: its heading, then a row per item.

use groove_controllers::AppState;
use groove_controllers::session_service::Living;
use groove_gfx::Rect;
use groove_types::{Task, Worktree, WorktreeDelivery};

use super::List;
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::style::Role;
use crate::views::session::worktree_row;
use crate::widget::{elide, hairline, row};

/// One line of a column: an item, a worktree under an open one, or the plan's divider.
pub(super) enum Line<'a> {
    Session(&'a Living),
    Worktree(&'a Worktree, Option<&'a WorktreeDelivery>),
    /// Its place in the plan, counted from one.
    Task(usize, &'a Task),
    Divider,
    Nothing(&'static str),
}

pub(super) fn draw(ctx: &mut Ctx, area: Rect, app: &AppState, ui: &Ui, list: List) {
    let lines = lines(app, ui, list);
    let head = Rect::new(area.x, area.y, area.w, ctx.tokens.header);
    heading(ctx, head, list, counted(&lines));
    edge(ctx, area);
    let body = Rect::new(area.x, head.bottom(), area.w, area.h - head.h);
    rows(ctx, body, app, ui, list, &lines);
}

/// What a column holds, top to bottom.
fn lines<'a>(app: &'a AppState, ui: &Ui, list: List) -> Vec<Line<'a>> {
    match list {
        List::Live => super::live::lines(app, ui),
        List::Next => super::plan::lines(app, ui),
        List::Review => vec![Line::Nothing("reviews arrive with the MRs")],
    }
}

/// How many items a column holds; its worktrees and its hints are not items.
fn counted(lines: &[Line<'_>]) -> usize {
    lines
        .iter()
        .filter(|line| matches!(line, Line::Session(_) | Line::Task(_, _)))
        .count()
}

fn heading(ctx: &mut Ctx, line: Rect, list: List, count: usize) {
    ctx.quad(line, ctx.styles.band());
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

/// The lines a column has room for, scrolled and clipped to it.
fn rows(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui, list: List, lines: &[Line<'_>]) {
    let heights = heights(&ctx.tokens, app, lines);
    let extent = (heights.iter().sum::<f32>() - body.h).max(0.0);
    ctx.scrolls(Scroller::Column(list as u8), extent);
    let scroll = ui.board.scroll(list).min(extent);
    ctx.clipped(body, |ctx| {
        let mut y = body.y - scroll;
        for (at, line) in lines.iter().enumerate() {
            let height = heights[at];
            if y + height >= body.y && y <= body.bottom() {
                let rect = Rect::new(body.x, y, body.w, height);
                one(ctx, rect, app, ui, line, closes(lines, at));
            }
            y += height;
        }
        if list == List::Next && ui.board.dragging.is_some() {
            super::plan::dragging(ctx, body, app, ui, scroll);
        }
    });
}

/// How tall a line of a column stands: one row, and one more line when it says why it
/// needs the user.
pub(super) fn heights(
    tokens: &crate::tokens::Tokens,
    app: &AppState,
    lines: &[Line<'_>],
) -> Vec<f32> {
    lines
        .iter()
        .map(|line| match reasons(app, line).is_empty() {
            true => item(tokens),
            false => item(tokens) + tokens.line,
        })
        .collect()
}

/// Why this line needs the user.
fn reasons<'a>(app: &'a AppState, line: &Line<'_>) -> &'a [groove_types::Attention] {
    match line {
        Line::Task(_, task) => app.task.needs(&task.external_id),
        _ => &[],
    }
}

/// How tall one plain line of a column is.
pub(super) fn item(tokens: &crate::tokens::Tokens) -> f32 {
    tokens.row + tokens.sm
}

/// Whether this line ends its item: the next one starts another, or there is none.
fn closes(lines: &[Line<'_>], at: usize) -> bool {
    !matches!(lines.get(at + 1), Some(Line::Worktree(_, _)))
}

fn one(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui, line: &Line<'_>, closes: bool) {
    match line {
        Line::Session(living) => super::live::session(ctx, rect, app, ui, living),
        Line::Worktree(worktree, delivery) => worktree_row::draw(ctx, rect, worktree, *delivery),
        Line::Task(at, task) => waiting(ctx, rect, app, ui, *at, task),
        Line::Divider => return super::plan::divider(ctx, rect),
        Line::Nothing(text) => {
            let style = ctx.styles.small(Role::Faint);
            return row(ctx, rect, ctx.tokens.md, text, style);
        }
    }
    if closes {
        hairline(ctx, rect, ctx.styles.line());
    }
}

/// One task waiting: its place in the plan, its title, what it is worth, and why it
/// needs the user.
fn waiting(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui, at: usize, task: &Task) {
    let target = Target::Task(task.short_id.clone());
    if ui.hover.as_ref() == Some(&target) {
        ctx.quad(rect, ctx.styles.hover());
    }
    ctx.hit(rect, target);
    let line = Rect::new(rect.x, rect.y, rect.w, item(&ctx.tokens));
    let start = place(ctx, line, at, task);
    let until = aside(ctx, line, &worth(task));
    named(ctx, line, &task.title, until, start);
    let reasons = app.task.needs(&task.external_id);
    if reasons.is_empty() {
        return;
    }
    let under = Rect::new(rect.x, line.bottom(), rect.w, ctx.tokens.line);
    let said = super::attention::said(reasons, groove_types::Timestamp::now());
    super::attention::draw(ctx, under, start, &said);
}

/// Where a task sits in the plan, and what a drag takes hold of. Returns the title's x.
fn place(ctx: &mut Ctx, line: Rect, at: usize, task: &Task) -> f32 {
    let style = ctx.styles.code(Role::Ghost);
    let text = at.to_string();
    let room = ctx.measure("00", &style);
    let width = ctx.measure(&text, &style);
    let box_ = Rect::new(line.x + ctx.tokens.md, line.y, room, line.h);
    row(ctx, box_, room - width, &text, style);
    ctx.hit(box_, Target::Place(task.external_id.clone()));
    box_.right() - line.x + ctx.tokens.sm
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
pub(super) fn aside(ctx: &mut Ctx, line: Rect, text: &str) -> f32 {
    if text.is_empty() {
        return line.right() - ctx.tokens.md;
    }
    let style = ctx.styles.small(Role::Faint);
    let width = ctx.measure(text, &style);
    let at = line.right() - ctx.tokens.md - width;
    row(ctx, Rect::new(at, line.y, width, line.h), 0.0, text, style);
    at
}

/// A row's title, from `start`, cut where its right-hand text begins.
pub(super) fn named(ctx: &mut Ctx, line: Rect, title: &str, until: f32, start: f32) {
    let style = ctx.styles.label(Role::Text);
    let room = (until - line.x - start - ctx.tokens.sm).max(0.0);
    let text = elide(ctx, title, &style, room);
    row(ctx, line, start, &text, style);
}
