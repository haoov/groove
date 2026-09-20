//! One line of a column: what it holds, how tall it stands, and how it is drawn.

use groove_controllers::AppState;
use groove_controllers::session_service::Living;
use groove_gfx::Rect;
use groove_types::{Task, Worktree, WorktreeDelivery};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::{elide, row};

/// One line of a column: an item, a worktree under an open one, or the plan's divider.
pub(super) enum Line<'a> {
    Session(&'a Living),
    Worktree(&'a Worktree, Option<&'a WorktreeDelivery>),
    /// Its place in the plan, counted from one.
    Task(usize, &'a Task),
    Divider,
    Nothing(&'static str),
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

/// How tall one plain line of a column is.
pub(super) fn item(tokens: &crate::tokens::Tokens) -> f32 {
    tokens.row + tokens.sm
}

/// One task waiting: its place in the plan, its title, what it is worth, and why it
/// needs the user.
pub(super) fn up_next(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui, at: usize, task: &Task) {
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
    let said = super::attention::line(reasons, groove_types::Timestamp::now());
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

/// Why this line needs the user.
fn reasons<'a>(app: &'a AppState, line: &Line<'_>) -> &'a [groove_types::Attention] {
    match line {
        Line::Task(_, task) => app.task.needs(&task.external_id),
        _ => &[],
    }
}
