//! The write the agent is waiting on, over the foot of its pane.

use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::Ask;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::style::Role;
use crate::widget::{elide, row, slot_at};

/// The oldest ask of the selected session, and how many stand behind it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let Some(activity) = app.agent.activity(&open.session.id) else {
        return;
    };
    let Some(ask) = activity.asks.first().cloned() else {
        return;
    };
    let band = band(ctx);
    ctx.quad(band, ctx.styles.raised());
    ctx.border(band, ctx.styles.line());
    ctx.hit(band, Target::Agent);
    let line = Rect::new(band.x, band.y + ctx.tokens.sm, band.w, ctx.tokens.row);
    let left = acts(ctx, ui, line, &ask);
    said(ctx, line, &ask, activity.asks.len(), left);
}

/// The band it stands in, at the foot of the agent's pane.
fn band(ctx: &Ctx) -> Rect {
    let pane = ctx.layout.agent;
    let height = ctx.tokens.row + ctx.tokens.sm * 2.0;
    Rect::new(
        pane.x + ctx.tokens.sm,
        pane.bottom() - height - ctx.tokens.sm,
        pane.w - ctx.tokens.sm * 2.0 - ctx.tokens.hairline,
        height,
    )
}

/// Approve and refuse, from the right. Returns where they start.
fn acts(ctx: &mut Ctx, ui: &Ui, line: Rect, ask: &Ask) -> f32 {
    let mut left = line.right();
    for (label, target, hovered) in [
        ("refuse", Target::Refuse(ask.id.clone()), Role::Bad),
        ("approve", Target::Approve(ask.id.clone()), Role::Ok),
    ] {
        let role = match ui.hover.as_ref() == Some(&target) {
            true => hovered,
            false => Role::Muted,
        };
        let style = ctx.styles.small(role);
        let word = ctx.measure(label, &style);
        let room = Rect::new(line.x, line.y, left - line.x, line.h);
        let at = left - word - ctx.tokens.sm * 2.0;
        let box_ = slot_at(ctx, room, at, word, Some(ctx.styles.ground()));
        row(ctx, box_, ctx.tokens.sm, label, style);
        ctx.hit(box_, target);
        left = box_.x - ctx.tokens.xs;
    }
    left
}

/// What the write is, cut to the room the buttons leave it.
fn said(ctx: &mut Ctx, line: Rect, ask: &Ask, waiting: usize, left: f32) {
    let style = ctx.styles.strong(Role::Attention);
    let at = ctx.tokens.sm;
    let word = ctx.measure(&ask.op, &style);
    row(ctx, line, at, &ask.op, style);
    let rest = ctx.styles.small(Role::Muted);
    let at = at + word + ctx.tokens.sm;
    let room = (left - line.x - at).max(0.0);
    let text = elide(ctx, &tail(ask, waiting), &rest, room);
    row(ctx, line, at, &text, rest);
}

/// The subject, and the writes still behind this one.
fn tail(ask: &Ask, waiting: usize) -> String {
    match (ask.subject.is_empty(), waiting) {
        (true, 1) => String::new(),
        (true, n) => format!("+{} more", n - 1),
        (false, 1) => ask.subject.clone(),
        (false, n) => format!("{} · +{} more", ask.subject, n - 1),
    }
}
