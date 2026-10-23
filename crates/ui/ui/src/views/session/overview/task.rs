//! What the overview shows of the task a session works: its properties on one line, its body.

use groove_gfx::{Edges, Rect};
use groove_types::{Task, TimeSummary};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::markdown::{blocks, laid, prose};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::Button;

/// The properties the task has a value for, in one line, then the hours left to log.
pub(super) fn properties(ctx: &mut Ctx, line: Rect, task: &Task, time: Option<TimeSummary>) {
    let (sm, md) = (ctx.tokens.sm, ctx.tokens.md);
    let mut room = line.pad(Edges::across(md, md));
    for (at, (said, role)) in said(task).into_iter().enumerate() {
        if at > 0 {
            Label::new("·", ctx.styles.body(Role::Ghost)).left(ctx, &mut room, sm);
        }
        Label::new(&said, ctx.styles.body(role)).left(ctx, &mut room, sm);
    }
    logging(ctx, &mut room, task, time);
}

/// Each property the task has a value for, with the role it reads in.
fn said(task: &Task) -> Vec<(String, Role)> {
    let dates = match (task.dates.start, task.dates.due) {
        (Some(start), Some(due)) => Some(format!("{start} → {due}")),
        (Some(start), None) => Some(format!("from {start}")),
        (None, Some(due)) => Some(format!("due {due}")),
        (None, None) => None,
    };
    let hours = match (task.logged, task.estimate) {
        (Some(logged), Some(estimate)) => Some(format!("{} of {}", hours(logged), hours(estimate))),
        (Some(logged), None) => Some(format!("{} logged", hours(logged))),
        (None, Some(estimate)) => Some(format!("{} estimated", hours(estimate))),
        (None, None) => None,
    };
    let status = (!task.status.is_empty()).then(|| task.status.clone());
    let priority = task.priority.map(|one| one.label().to_string());
    [
        (status, Role::Text),
        (priority, Role::Muted),
        (dates, Role::Muted),
        (hours, Role::Text),
    ]
    .into_iter()
    .filter_map(|(one, role)| one.map(|one| (one, role)))
    .collect()
}

/// What hands the source the hours the clock measured and nobody logged yet.
fn logging(ctx: &mut Ctx, room: &mut Rect, task: &Task, time: Option<TimeSummary>) {
    let left = time.map(TimeSummary::unlogged_hours).unwrap_or_default();
    if left <= 0.0 {
        return;
    }
    let target = Target::LogHours(task.external_id.clone());
    let (band, hover) = (ctx.styles.band(), ctx.styles.glass_hover());
    let label = format!("log {}", hours(left));
    let log = Button::new(&label, target, Role::Working, band)
        .flat()
        .hover(hover);
    log.left(ctx, room, ctx.tokens.sm);
}

/// How tall the body stands as Markdown `wide` across.
pub(super) fn body_height(ctx: &mut Ctx, text: &str, wide: f32) -> f32 {
    let inner = wide - ctx.tokens.xl * 2.0;
    laid(ctx, &blocks(text), inner, Role::Muted).height
}

/// The body as Markdown, a padding all around.
pub(super) fn body(ctx: &mut Ctx, area: Rect, card: Rect, text: &str) {
    let at = card.pad(Edges::all(ctx.tokens.md));
    let link = |url: &str| Target::Link(url.to_string());
    prose(ctx, (at, area), text, Role::Muted, link);
}

fn hours(value: f32) -> String {
    format!("{value}h")
}
