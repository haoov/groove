//! Thinking, looks, commands and MCP calls folded into one row, with their card under it.

use groove_gfx::{Align, Edges, Rect};
use groove_types::Hue;
use groove_types::chat::Glance;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{from_left, from_top};
use groove_ui_kit::text::{Label, lasted};
use groove_ui_kit::widgets::{busy, icon};

use super::commands;
use crate::ctx::Ctx;

pub(super) fn height(ctx: &Ctx, glance: &Glance, opened: bool, width: f32) -> f32 {
    if glance.ran.is_empty() {
        return ctx.tokens.row;
    }
    let card = commands::height(ctx, &glance.ran, opened, width - lead(ctx));
    ctx.tokens.row + ctx.tokens.xs + card
}

/// The room the card leaves under the row's first mark.
fn lead(ctx: &Ctx) -> f32 {
    ctx.tokens.icon + ctx.tokens.xs
}

/// The row, its last mark turning while `live`, then the commands under it, whole when `opened`.
pub(super) fn draw(ctx: &mut Ctx, at: Rect, glance: &Glance, (live, opened): (bool, bool)) {
    let mut room = at;
    let mut head = from_top(&mut room, ctx.tokens.row, ctx.tokens.xs);
    if let Some(took) = took(ctx, glance, live) {
        Label::new(&took, ctx.styles.code(Role::Ghost)).right(ctx, &mut head, ctx.tokens.sm);
    }
    let parts = parts(glance);
    let last = parts.len().saturating_sub(1);
    for (at, (mark, role, said)) in parts.into_iter().enumerate() {
        if at > 0 {
            Label::new("·", ctx.styles.body(Role::Ghost)).left(ctx, &mut head, ctx.tokens.sm);
        }
        let size = ctx.tokens.icon;
        let slot = from_left(&mut head, size, ctx.tokens.xs);
        let slot = slot.align((size, size), Align::Center, Align::Center);
        match live && at == last {
            true => busy(ctx, slot, role),
            false => icon(ctx, slot, mark, role),
        }
        Label::new(&said, ctx.styles.body(Role::Muted)).left(ctx, &mut head, ctx.tokens.sm);
    }
    let summary = match (&glance.doing, glance.ran.is_empty()) {
        (Some(doing), _) => doing.clone(),
        (None, true) => glance.looked.join(", "),
        (None, false) => String::new(),
    };
    Label::new(&summary, ctx.styles.small(Role::Faint)).draw(ctx, head);
    if glance.ran.is_empty() {
        return;
    }
    commands::draw(
        ctx,
        room.pad(Edges::across(lead(ctx), 0.0)),
        &glance.ran,
        opened,
    );
}

/// Whether the commands' card is open: by its first command.
pub(super) fn opened(glance: &Glance, opened: &[String]) -> bool {
    let first = glance.ran.first().map(|one| &one.id);
    first.is_some_and(|id| opened.contains(id))
}

/// Each kind glanced: its mark, its colour and its count.
fn parts(glance: &Glance) -> Vec<(Mark, Role, String)> {
    let mut parts = Vec::new();
    if glance.thoughts > 0 {
        parts.push((Mark::Thought, Role::Hue(Hue::Mauve), "thought".to_string()));
    }
    if !glance.looked.is_empty() {
        let said = format!(
            "read {}",
            super::entry::counted(glance.looked.len(), "file")
        );
        parts.push((Mark::Looked, Role::Hue(Hue::Sapphire), said));
    }
    let shell = glance.ran.iter().filter(|one| one.tool.is_none()).count();
    if shell > 0 {
        parts.push((Mark::Terminal, Role::Attention, format!("bash {shell}")));
    }
    let called = glance.ran.len() - shell;
    if called > 0 {
        parts.push((Mark::Agent, Role::Accent, format!("mcp {called}")));
    }
    parts
}

/// How long the glance took: to the record after it, or to now while it runs.
fn took(ctx: &Ctx, glance: &Glance, live: bool) -> Option<String> {
    let since = glance.since?;
    let until = if live { ctx.now } else { glance.until? };
    let took = since.age_at(until);
    (!took.is_zero()).then(|| lasted(took))
}
