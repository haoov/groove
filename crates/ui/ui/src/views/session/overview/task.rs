//! What the overview shows of the task a session works: its properties in columns, its body.

use groove_gfx::{Edges, Rect};
use groove_types::{StatusIntent, Task};

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::markdown::{blocks, laid, prose};

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
