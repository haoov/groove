//! One line of a column: what it holds, how tall it stands, and how it is drawn.

use groove_controllers::AppState;
use groove_controllers::session_service::Living;
use groove_gfx::{Edges, Rect};
use groove_types::{Priority, Task, Worktree, WorktreeDelivery};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{hoverable, square};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::icon;

/// One line of a column: an item, a worktree under an open one, or the plan's divider.
pub(super) enum Line<'a> {
    Session(&'a Living),
    Worktree(&'a Worktree, Option<WorktreeDelivery>),
    Task(&'a Task),
    /// One MR the forge asks this user to review.
    Review(&'a groove_types::ReviewMr),
    Divider,
    Nothing(&'static str),
}

/// How tall a line stands: one row, and one more when it carries a second line.
pub(super) fn heights(
    tokens: &groove_ui_kit::base::tokens::Tokens,
    app: &AppState,
    lines: &[Line<'_>],
) -> Vec<f32> {
    lines
        .iter()
        .map(|line| match under(app, line) {
            true => item(tokens) + tokens.line,
            false => item(tokens),
        })
        .collect()
}

/// Whether this line carries a second line under its title.
fn under(app: &AppState, line: &Line<'_>) -> bool {
    match line {
        Line::Review(_) => true,
        _ => !reasons(app, line).is_empty(),
    }
}

/// How tall one plain line of a column is.
pub(super) fn item(tokens: &groove_ui_kit::base::tokens::Tokens) -> f32 {
    tokens.row + tokens.sm
}

/// One task waiting: its source, its title, its worth, and why it needs the user.
pub(super) fn up_next(ctx: &mut Ctx, rect: Rect, app: &AppState, task: &Task) {
    hoverable(ctx, rect, Target::Task(task.short_id.clone()));
    let mut rest = rect;
    let line = rest.take_top(item(&ctx.tokens));
    let mut room = line.pad(Edges::across(ctx.tokens.md, 0.0));
    handle(ctx, &mut room, task);
    let start = room.x;
    worth(ctx, &mut room, task);
    named(ctx, room, &task.title);
    let reasons = app.task.needs(&task.external_id);
    if !reasons.is_empty() {
        let under = rest.take_top(ctx.tokens.line);
        let said = super::attention::line(reasons, groove_types::Timestamp::now());
        super::attention::draw(ctx, under.pad(Edges::across(start - under.x, 0.0)), &said);
    }
}

/// The mark of the task's source, which a drag takes hold of to move it in the plan.
fn handle(ctx: &mut Ctx, room: &mut Rect, task: &Task) {
    let size = ctx.tokens.icon;
    let mark = square(room.take_left(size), size);
    room.take_left(ctx.tokens.sm);
    icon(ctx, mark, Mark::of_source(task.provider), Role::Faint);
    ctx.hit(mark, Target::Place(task.external_id.clone()));
}

/// The priority, as the row's right-hand text in its level's colour.
fn worth(ctx: &mut Ctx, room: &mut Rect, task: &Task) {
    let Some(level) = task.priority else {
        return aside(ctx, room, "");
    };
    let role = match level {
        Priority::High => Role::Bad,
        Priority::Medium => Role::Warn,
        Priority::Low => Role::Ok,
    };
    aside_in(ctx, room, level.label(), role);
}

/// A row's right-hand text, at the right of `room`.
pub(super) fn aside(ctx: &mut Ctx, room: &mut Rect, text: &str) {
    aside_in(ctx, room, text, Role::Faint);
}

fn aside_in(ctx: &mut Ctx, room: &mut Rect, text: &str, role: Role) {
    room.take_right(ctx.tokens.md);
    if !text.is_empty() {
        Label::new(text, ctx.styles.small(role)).right(ctx, room, 0.0);
    }
}

/// A row's title, a gap short of its right-hand text.
pub(super) fn named(ctx: &mut Ctx, mut room: Rect, title: &str) {
    room.take_right(ctx.tokens.sm);
    Label::new(title, ctx.styles.label(Role::Text)).draw(ctx, room);
}

/// Why this line needs the user.
fn reasons<'a>(app: &'a AppState, line: &Line<'_>) -> &'a [groove_types::Attention] {
    match line {
        Line::Task(task) => app.task.needs(&task.external_id),
        _ => &[],
    }
}
