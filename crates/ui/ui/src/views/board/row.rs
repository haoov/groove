//! One line of a column: what it holds, how tall it stands, and how it is drawn.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::{Priority, Task};
use groove_ui_kit::shape::{from_right, from_top, span};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Badge, card_inset, glass_card_row, lead};

/// One line of Up next: a task.
pub(super) enum Line<'a> {
    Task(&'a Task),
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
            true => card(tokens),
            false => item(tokens) + card_inset(tokens) * 2.0,
        })
        .collect()
}

/// Whether this line carries a second line under its title.
fn under(app: &AppState, line: &Line<'_>) -> bool {
    let Line::Task(task) = line;
    task.priority.is_some() || task.project.is_some() || !reasons(app, line).is_empty()
}

/// How tall one plain line of a column is.
pub(super) fn item(tokens: &groove_ui_kit::base::tokens::Tokens) -> f32 {
    tokens.row + tokens.sm
}

/// How tall a card stands with a second line under its title, its inset included.
pub fn card(tokens: &groove_ui_kit::base::tokens::Tokens) -> f32 {
    lines(tokens) + card_inset(tokens) * 2.0
}

/// How tall the title and the second line stand together.
pub(super) fn lines(tokens: &groove_ui_kit::base::tokens::Tokens) -> f32 {
    item(tokens) + tokens.line + tokens.xs
}

/// One task waiting: its source and title, then its worth and why it needs the user.
pub(super) fn up_next(ctx: &mut Ctx, rect: Rect, app: &AppState, task: &Task) {
    glass_card_row(ctx, rect, Some(Target::Task(task.short_id.clone())), false);
    let mut rest = rect.pad(Edges::xy(0.0, card_inset(&ctx.tokens)));
    let line = from_top(&mut rest, item(&ctx.tokens), 0.0);
    let mut room = line.pad(Edges::across(ctx.tokens.md, 0.0));
    handle(ctx, &mut room, task);
    let start = room.x;
    aside(ctx, &mut room, "");
    named(ctx, room, &task.title);
    let reasons = app.task.needs(&task.external_id);
    if !under(app, &Line::Task(task)) {
        return;
    }
    let under = from_top(&mut rest, ctx.tokens.line, 0.0);
    let x = placed(ctx, under, start, task);
    let x = worth(ctx, under, x, task);
    if !reasons.is_empty() {
        let said = super::attention::line(reasons, ctx.now);
        super::attention::draw(ctx, under.pad(Edges::across(x - under.x, 0.0)), &said);
    }
}

/// The mark of the task's source, which a drag takes hold of to move it in the plan.
fn handle(ctx: &mut Ctx, room: &mut Rect, task: &Task) {
    let mark = lead(ctx, room, Mark::of_source(task.provider), Role::Faint);
    ctx.hit(mark, Target::Place(task.external_id.clone()));
}

/// The project from `x`, short of the badge after it; returns the x after it.
fn placed(ctx: &mut Ctx, line: Rect, x: f32, task: &Task) -> f32 {
    let Some(project) = &task.project else {
        return x;
    };
    let badge = badge(task).map_or(0.0, |one| one.width(ctx) + ctx.tokens.sm);
    let style = ctx.styles.small(Role::Faint);
    let room = line.right() - x - ctx.tokens.md - badge;
    let project = groove_ui_kit::text::elide(ctx, project, &style, room);
    let wide = ctx.measure(&project, &style);
    Label::new(&project, style).draw(ctx, span(line, x, wide));
    x + wide + ctx.tokens.sm
}

/// The priority as a badge from `x`; returns the x after it.
fn worth(ctx: &mut Ctx, line: Rect, x: f32, task: &Task) -> f32 {
    match badge(task) {
        Some(badge) => badge.at(ctx, line, x).right() + ctx.tokens.sm,
        None => x,
    }
}

/// The priority as a badge in its level's colour.
fn badge(task: &Task) -> Option<Badge<'static>> {
    let level = task.priority?;
    let role = match level {
        Priority::High => Role::Bad,
        Priority::Medium => Role::Warn,
        Priority::Low => Role::Ok,
    };
    Some(Badge::new(level.label(), role))
}

/// A row's right-hand text, at the right of `room`.
pub(super) fn aside(ctx: &mut Ctx, room: &mut Rect, text: &str) {
    from_right(room, ctx.tokens.md, 0.0);
    if !text.is_empty() {
        Label::new(text, ctx.styles.small(Role::Faint)).right(ctx, room, 0.0);
    }
}

/// A row's title, a gap short of its right-hand text.
pub(super) fn named(ctx: &mut Ctx, mut room: Rect, title: &str) {
    from_right(&mut room, ctx.tokens.sm, 0.0);
    Label::new(title, ctx.styles.label(Role::Text)).draw(ctx, room);
}

/// Why this line needs the user.
fn reasons<'a>(app: &'a AppState, line: &Line<'_>) -> &'a [groove_types::Attention] {
    let Line::Task(task) = line;
    app.task.needs(&task.external_id)
}
