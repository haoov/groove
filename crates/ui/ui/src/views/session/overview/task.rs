//! What the overview shows of the task a session works: its properties in columns, its body.

use groove_gfx::{Edges, Rect};
use groove_types::{StatusIntent, Task};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::markdown::{blocks, laid, prose};
use groove_ui_kit::shape::{band, from_left, side_rule};
use groove_ui_kit::text::Label;

/// The properties the task has a value for, in columns across the card: each name over its value,
/// a hairline between them.
pub(super) fn properties(ctx: &mut Ctx, card: Rect, task: &Task, maintenance: bool) {
    let said = said(task, maintenance);
    if said.is_empty() {
        return;
    }
    let (md, row) = (ctx.tokens.md, ctx.tokens.row);
    let wide = widths(ctx, &said, card.w);
    let mut room = card;
    for (at, ((name, value, role), wide)) in said.into_iter().zip(wide).enumerate() {
        let column = from_left(&mut room, wide, 0.0);
        if at > 0 {
            side_rule(ctx, column, column.x, ctx.styles.line());
        }
        let inner = column.pad(Edges::across(md, md));
        let named = band(inner, ctx.tokens.sm, row);
        let style = ctx.styles.heading(Role::Ghost);
        Label::new(&name.to_uppercase(), style).draw(ctx, named);
        let shown = band(inner, ctx.tokens.sm + row, row);
        Label::new(&value, ctx.styles.body(role)).draw(ctx, shown);
    }
}

/// Each column as wide as its name or its value, padded, the room left shared out evenly.
fn widths(ctx: &mut Ctx, said: &[(&str, String, Role)], room: f32) -> Vec<f32> {
    let pad = ctx.tokens.md * 2.0;
    let (heading, body) = (ctx.styles.heading(Role::Ghost), ctx.styles.body(Role::Text));
    let natural: Vec<f32> = said
        .iter()
        .map(|(name, value, _)| {
            let named = ctx.measure(&name.to_uppercase(), &heading);
            named.max(ctx.measure(value, &body)) + pad
        })
        .collect();
    let spare = (room - natural.iter().sum::<f32>()) / natural.len().max(1) as f32;
    natural
        .into_iter()
        .map(|one| (one + spare).max(0.0))
        .collect()
}

/// Each property the task has a value for: its name, its value, the role it reads in.
pub(super) fn said(task: &Task, maintenance: bool) -> Vec<(&'static str, String, Role)> {
    let dates = match (task.dates.start, task.dates.due) {
        (Some(start), Some(due)) => Some(format!("{start} → {due}")),
        (Some(start), None) => Some(format!("from {start}")),
        (None, Some(due)) => Some(format!("due {due}")),
        (None, None) => None,
    };
    let status = (!task.status.is_empty()).then(|| task.status.clone());
    let priority = task.priority.map(|one| one.label().to_string());
    [
        ("status", status, intended(task.intent)),
        ("priority", priority, Role::Text),
        ("dates", dates, Role::Text),
        ("hours", task.logged.map(hours), Role::Text),
        (
            "maintenance",
            maintenance.then(|| "on".to_string()),
            Role::Accent,
        ),
    ]
    .into_iter()
    .filter_map(|(name, one, role)| one.map(|one| (name, one, role)))
    .collect()
}

/// A status's colour: blue in progress, green done, muted ready.
fn intended(intent: Option<StatusIntent>) -> Role {
    match intent {
        Some(StatusIntent::InProgress) => Role::Working,
        Some(StatusIntent::Done) => Role::Ok,
        Some(StatusIntent::Ready) => Role::Muted,
        None => Role::Text,
    }
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
