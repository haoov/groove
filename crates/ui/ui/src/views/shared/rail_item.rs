use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::Rect;
use groove_types::{AgentStatus, Ask, AttentionClass, SessionId};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{after_mark, ago, box_in, elide, hairline, icon, leading, row, slot_at, turn};
use crate::{Surface, Ui};

/// The row's lines: head, state, and the answers while the session asks.
pub fn height(ctx: &Ctx, app: &AppState, id: &SessionId) -> f32 {
    let base = (ctx.tokens.sm + ctx.tokens.line) * 2.0 + ctx.tokens.xs;
    match asked(app, id) {
        Some(_) => base + ctx.tokens.row,
        None => base,
    }
}

/// Type icon, title and how long it has waited, then the agent's state under them.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect, open: &Open) {
    let id = &open.session.id;
    ground(ctx, rect, app, ui, id);
    ctx.hit(rect, Target::Session(id.clone()));

    let head = Rect::new(rect.x, rect.y + ctx.tokens.sm, rect.w, ctx.tokens.line);
    let box_ = leading(ctx, head, ctx.tokens.md);
    icon(ctx, box_, Mark::of_kind(&open.session.kind), Role::Faint);
    let until = waited(ctx, app, head, id);
    title(ctx, head, &open.session.title, until);

    let under = Rect::new(rect.x, head.bottom() + ctx.tokens.xs, rect.w, head.h);
    match asked(app, id) {
        Some((ask, waiting)) => asking(ctx, ui, under, &ask, waiting),
        None => state(ctx, app, under, open),
    }

    let rule = ctx.styles.line();
    hairline(ctx, rect, rule);
}

/// The first write the session waits on, and how many wait in all.
fn asked(app: &AppState, id: &SessionId) -> Option<(Ask, usize)> {
    let asks = &app.agent.activity(id)?.asks;
    Some((asks.first()?.clone(), asks.len()))
}

/// The write in peach where the state stands, and its two answers on the line under it.
fn asking(ctx: &mut Ctx, ui: &Ui, line: Rect, ask: &Ask, waiting: usize) {
    let style = ctx.styles.small(Role::Attention);
    let indent = after_mark(ctx, ctx.tokens.md);
    let verb = groove_controllers::agent_service::tools::verb(&ask.op);
    let label = match waiting {
        1 => format!("asks to {verb}"),
        n => format!("asks to {verb} · +{}", n - 1),
    };
    let room = (line.w - indent - ctx.tokens.md).max(0.0);
    let label = elide(ctx, &label, &style, room);
    row(ctx, line, indent, &label, style);
    let under = Rect::new(line.x, line.bottom(), line.w, ctx.tokens.row);
    answers(ctx, ui, under, ask, line.x + indent);
}

/// Approve, then review, from where the text starts.
fn answers(ctx: &mut Ctx, ui: &Ui, line: Rect, ask: &Ask, from: f32) {
    let mut at = from;
    for (label, target, role) in [
        ("Approve", Target::Approve(ask.id.clone()), Role::Attention),
        ("Review", Target::Examine(ask.id.clone()), Role::Muted),
    ] {
        let hovered = ui.hover.as_ref() == Some(&target);
        let style = ctx.styles.small(match (hovered, role) {
            (true, Role::Muted) => Role::Text,
            _ => role,
        });
        let word = ctx.measure(label, &style);
        let box_ = slot_at(ctx, line, at, word, Some(ctx.styles.ground()));
        row(ctx, box_, ctx.tokens.sm, label, style);
        ctx.hit(box_, target);
        at = box_.right() + ctx.tokens.xs;
    }
}

/// A hovered row is raised, and a selected one while the session surface is up.
fn ground(ctx: &mut Ctx, rect: Rect, app: &AppState, ui: &Ui, id: &SessionId) {
    let hovered = ui.hover.as_ref() == Some(&Target::Session(id.clone()));
    let selected = ui.surface == Surface::Session && app.session.selected.as_ref() == Some(id);
    if selected || hovered {
        let raised = ctx.styles.raised();
        ctx.quad(rect, raised);
    }
}

/// The title, cut with an ellipsis where the time starts.
fn title(ctx: &mut Ctx, head: Rect, text: &str, until: f32) {
    let style = ctx.styles.label(Role::Text);
    let indent = after_mark(ctx, ctx.tokens.md);
    let room = (until - head.x - indent - ctx.tokens.sm).max(0.0);
    let text = elide(ctx, text, &style, room);
    let line = Rect::new(head.x, head.y, indent + room, head.h);
    row(ctx, line, indent, &text, style);
}

/// How long the agent has waited, right-aligned. Returns where it starts.
fn waited(ctx: &mut Ctx, app: &AppState, head: Rect, id: &SessionId) -> f32 {
    let right = head.right() - ctx.tokens.md;
    let Some(activity) = app.agent.activity(id) else {
        return right;
    };
    let style = ctx.styles.small(Role::Ghost);
    let text = ago(activity.changed_at.age_at(ctx.now));
    let width = ctx.measure(&text, &style);
    let at = right - width;
    row(ctx, Rect::new(at, head.y, width, head.h), 0.0, &text, style);
    at
}

/// The state line. A working agent's mark turns; every other state stands still.
fn state(ctx: &mut Ctx, app: &AppState, rect: Rect, open: &Open) {
    let (label, role) = state_of(app, open);
    let style = ctx.styles.small(role);
    let indent = after_mark(ctx, ctx.tokens.md);
    if role != Role::Working {
        return row(ctx, rect, indent, &label, style);
    }
    let box_ = box_in(rect, rect.x + indent, style.size);
    ctx.icon(box_, Mark::Busy, turn(ctx.tick), style.color);
    row(
        ctx,
        rect,
        indent + style.size + ctx.tokens.xs,
        &label,
        style,
    );
}

/// What the agent is doing, and its colour.
fn state_of(app: &AppState, open: &Open) -> (String, Role) {
    let Some(activity) = app.agent.activity(&open.session.id) else {
        return ("idle".into(), Role::Ghost);
    };
    let label = match &activity.status {
        _ if !activity.asks.is_empty() => asks(activity.asks.len()),
        AgentStatus::Working => "working".into(),
        AgentStatus::Done { .. } => "done".into(),
        AgentStatus::Idle => "idle".into(),
        AgentStatus::Exited { code } => format!("exited {code}"),
        AgentStatus::Error { message } => format!("error: {message}"),
    };
    let role = match activity.class() {
        AttentionClass::NeedsYou => Role::Attention,
        AttentionClass::Moving => Role::Working,
        AttentionClass::ActWhenYouLook => Role::Text,
        AttentionClass::Quiet => Role::Ghost,
    };
    (label, role)
}

fn asks(count: usize) -> String {
    match count {
        1 => "asks to write".into(),
        n => format!("asks {n} writes"),
    }
}
