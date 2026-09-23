//! The agent's own row, under its screen: the skills it can be sent and a reload, or
//! the write it is waiting on with the two answers to it.

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;
use groove_types::Ask;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{box_in, elide, row, slot_at};

/// What the agent waits on, or what it can be sent.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let line = ctx.layout.agent_bar;
    ctx.quad(line, ctx.styles.band());
    let rule = Rect::new(line.x, line.y, line.w, ctx.tokens.hairline);
    ctx.quad(rule, ctx.styles.line());
    let asks = app
        .agent
        .activity(&open.session.id)
        .map(|one| one.asks.len())
        .unwrap_or_default();
    match asked(app, open) {
        Some(ask) => waiting(ctx, line, asks, ui, &ask),
        None => offered(ctx, line, app, ui, open),
    }
}

/// The write this session's agent is waiting on.
fn asked(app: &AppState, open: &Open) -> Option<Ask> {
    app.agent.activity(&open.session.id)?.asks.first().cloned()
}

/// The write, and the two answers to it.
fn waiting(ctx: &mut Ctx, line: Rect, waiting: usize, ui: &Ui, ask: &Ask) {
    let left = acts(ctx, line, ui, ask);
    let style = ctx.styles.strong(Role::Attention);
    let at = ctx.tokens.md;
    let width = ctx.measure(&ask.op, &style);
    row(ctx, line, at, &ask.op, style);
    let at = at + width + ctx.tokens.sm;
    let rest = ctx.styles.small(Role::Muted);
    let room = (left - line.x - at).max(0.0);
    let text = elide(ctx, &tail(ask, waiting), &rest, room);
    row(ctx, line, at, &text, rest);
}

/// Approve and refuse, from the row's own end. Returns where they start.
fn acts(ctx: &mut Ctx, line: Rect, ui: &Ui, ask: &Ask) -> f32 {
    let mut left = line.right();
    for (label, target, lit) in [
        ("refuse", Target::Refuse(ask.id.clone()), Role::Bad),
        ("approve", Target::Approve(ask.id.clone()), Role::Ok),
    ] {
        let role = match ui.hover.as_ref() == Some(&target) {
            true => lit,
            false => Role::Muted,
        };
        let style = ctx.styles.small(role);
        let word = ctx.measure(label, &style);
        let room = Rect::new(line.x, line.y, left - line.x, line.h);
        let at = left - word - ctx.tokens.sm * 2.0 - ctx.tokens.xs;
        let box_ = slot_at(ctx, room, at, word, Some(ctx.styles.ground()));
        row(ctx, box_, ctx.tokens.sm, label, style);
        ctx.hit(box_, target);
        left = box_.x - ctx.tokens.xs;
    }
    left
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

/// The skills menu and the reload, from the row's own end.
fn offered(ctx: &mut Ctx, line: Rect, app: &AppState, ui: &Ui, open: &Open) {
    let id = open.session.id.clone();
    let stale = app.agent.stale(&id);
    let mut left = line.right();
    for (label, target, role, caret) in [
        (
            "reload",
            Target::Reload(id.clone()),
            reload_role(stale),
            false,
        ),
        ("skills", Target::Skills(id.clone()), Role::Muted, true),
    ] {
        let hovered = ui.hover.as_ref() == Some(&target);
        let style = ctx.styles.small(match hovered {
            true => Role::Text,
            false => role,
        });
        let word = ctx.measure(label, &style);
        let held = match caret {
            true => word + ctx.tokens.xs + ctx.tokens.small,
            false => word,
        };
        let room = Rect::new(line.x, line.y, left - line.x, line.h);
        let at = left - held - ctx.tokens.sm * 2.0 - ctx.tokens.xs;
        let box_ = slot_at(ctx, room, at, held, Some(ctx.styles.ground()));
        row(ctx, box_, ctx.tokens.sm, label, style);
        if caret {
            let size = ctx.tokens.small;
            let mark = box_in(box_, box_.right() - ctx.tokens.sm - size, size);
            ctx.icon(mark, Mark::Down, Mark::UPWARDS, style.color);
        }
        ctx.hit(box_, target);
        left = box_.x - ctx.tokens.xs;
    }
    said(ctx, line, app, open, stale);
}

fn counted(skills: usize) -> String {
    match skills {
        1 => "1 skill".to_string(),
        n => format!("{n} skills"),
    }
}

fn reload_role(stale: bool) -> Role {
    match stale {
        true => Role::Attention,
        false => Role::Muted,
    }
}

/// What the row says when nothing waits: the agent's own name for itself.
fn said(ctx: &mut Ctx, line: Rect, app: &AppState, open: &Open, stale: bool) {
    let size = ctx.tokens.small;
    let box_ = box_in(line, line.x + ctx.tokens.md, size);
    ctx.icon(box_, Mark::Busy, 0, ctx.styles.color(Role::Faint));
    let style = ctx.styles.small(Role::Faint);
    let at = ctx.tokens.md + size + ctx.tokens.xs;
    let count = counted(app.agent.skills_for(&open.session.kind).len());
    let text = match stale {
        true => format!("{count} · one is newer than the agent"),
        false => count,
    };
    row(ctx, line, at, &text, style);
}
