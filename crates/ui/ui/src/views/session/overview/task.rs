//! What the overview shows of the task a session works: its properties, then its body.

use groove_gfx::Rect;
use groove_types::Task;

use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::{Row, list, row, wrapped};

const UNSET: &str = "—";

/// The six properties, each on its own line. Returns the y under the last.
pub(super) fn properties(ctx: &mut Ctx, area: Rect, top: f32, task: &Task) -> f32 {
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
    list(
        ctx,
        Rect::new(area.x, top, area.w, ctx.tokens.row),
        &rows,
        None,
    )
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
