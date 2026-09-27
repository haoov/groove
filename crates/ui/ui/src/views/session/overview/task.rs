//! What the overview shows of the task a session works: its properties, then its body.

use groove_gfx::Rect;
use groove_types::{Task, TimeSummary};

use crate::Ui;
use crate::base::ctx::Ctx;
use crate::base::hit::Target;
use crate::base::style::Role;
use crate::text::{row, wrapped};
use crate::widgets::{Row, button, list};

const UNSET: &str = "—";

/// The six properties a line each, the hours with the clock's; returns the y under them.
pub(super) fn properties(
    ctx: &mut Ctx,
    area: Rect,
    top: f32,
    task: &Task,
    time: Option<TimeSummary>,
    ui: &Ui,
) -> f32 {
    let (label, value) = (ctx.styles.body(Role::Faint), ctx.styles.body(Role::Text));
    let at = ctx.tokens.aside_near + ctx.tokens.md;
    let held = [
        ("Status", text(said(&task.status))),
        ("Priority", text(task.priority.map(|one| one.label()))),
        ("Start", text(task.dates.start)),
        ("Due", text(task.dates.due)),
        ("Estimate", text(task.estimate.map(hours))),
        ("Logged", text(task.logged.map(hours))),
    ];
    let rows: Vec<Row<'_>> = held
        .iter()
        .map(|(name, held)| Row::new(ctx.tokens.md, name, label).aside(at, held, value))
        .collect();
    let bottom = list(
        ctx,
        Rect::new(area.x, top, area.w, ctx.tokens.row),
        &rows,
        None,
    );
    let hours = Rect::new(area.x, bottom - ctx.tokens.row, area.w, ctx.tokens.row);
    logging(ctx, hours, task, time, ui);
    bottom
}

/// What hands the source the hours the clock measured, at the end of their own line.
fn logging(ctx: &mut Ctx, line: Rect, task: &Task, time: Option<TimeSummary>, ui: &Ui) {
    let left = time.map(TimeSummary::unlogged_hours).unwrap_or_default();
    if left <= 0.0 {
        return;
    }
    let target = Target::LogHours(task.external_id.clone());
    let ground = match ui.hover.as_ref() == Some(&target) {
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

/// The body as text, wrapped to the area's width. Returns the y under the last line.
pub(super) fn body(ctx: &mut Ctx, area: Rect, top: f32, text: &str) -> f32 {
    let style = ctx.styles.body(Role::Muted);
    let pad = ctx.tokens.md;
    let height = ctx.tokens.line;
    let lines = wrapped(ctx, text.trim(), &style, area.w - pad * 2.0);
    let mut y = top;
    for line in &lines {
        if y + height >= area.y && y <= area.bottom() {
            let at = Rect::new(area.x, y, area.w, height);
            row(ctx, at, pad, line, style);
        }
        y += height;
    }
    y
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
