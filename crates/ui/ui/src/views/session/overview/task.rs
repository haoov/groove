//! What the overview shows of the task a session works: its properties, then its body.

use groove_gfx::Rect;
use groove_types::{Task, TimeSummary};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::{row, wrapped};
use groove_ui_kit::widgets::button;

const UNSET: &str = "—";

/// The six properties a line each, the hours with the clock's.
pub(super) fn properties(ctx: &mut Ctx, column: &mut Rect, task: &Task, time: Option<TimeSummary>) {
    let held = [
        ("Status", text(said(&task.status))),
        ("Priority", text(task.priority.map(|one| one.label()))),
        ("Start", text(task.dates.start)),
        ("Due", text(task.dates.due)),
        ("Estimate", text(task.estimate.map(hours))),
        ("Logged", text(task.logged.map(hours))),
    ];
    let held: Vec<(&str, &str)> = held
        .iter()
        .map(|(name, one)| (*name, one.as_str()))
        .collect();
    let mut taken = super::table(ctx, column, &held);
    logging(ctx, taken.take_bottom(ctx.tokens.row), task, time);
}

/// What hands the source the hours the clock measured, at the end of their own line.
fn logging(ctx: &mut Ctx, line: Rect, task: &Task, time: Option<TimeSummary>) {
    let left = time.map(TimeSummary::unlogged_hours).unwrap_or_default();
    if left <= 0.0 {
        return;
    }
    let target = Target::LogHours(task.external_id.clone());
    let ground = match ctx.hovered(&target) {
        true => ctx.styles.hover(),
        false => ctx.styles.band(),
    };
    let label = format!("log {}", hours(left));
    let box_ = button(
        ctx,
        line,
        &label,
        ctx.styles.small(Role::Working),
        Some(ground),
    );
    ctx.hit(box_, target);
}

/// The body as text, wrapped to the area's width.
pub(super) fn body(ctx: &mut Ctx, area: Rect, column: &mut Rect, text: &str) {
    let style = ctx.styles.body(Role::Muted);
    let pad = ctx.tokens.md;
    let lines = wrapped(ctx, text.trim(), &style, column.w - pad * 2.0);
    for line in &lines {
        let at = column.take_top(ctx.tokens.line);
        if at.bottom() >= area.y && at.y <= area.bottom() {
            row(ctx, at, pad, line, style);
        }
    }
}

fn hours(value: f32) -> String {
    format!("{value}h")
}

fn text(value: Option<impl ToString>) -> String {
    value.map_or_else(|| UNSET.to_string(), |one| one.to_string())
}

fn said(status: &str) -> Option<&str> {
    (!status.is_empty()).then_some(status)
}
