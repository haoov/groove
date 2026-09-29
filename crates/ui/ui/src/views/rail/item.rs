use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{AgentStatus, Ask, AttentionClass, SessionId};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::{Surface, Ui};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::motion::turn;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{after_mark, hairline, ruled, square};
use groove_ui_kit::text::{Label, ago};
use groove_ui_kit::widgets::{Button, icon};

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
    if ctx.hovered(&Target::Session(id.clone())) {
        ctx.quad(rect, ctx.styles.hover());
    }
    ctx.hit(rect, Target::Session(id.clone()));
    let (xs, sm, md, size) = (ctx.tokens.xs, ctx.tokens.sm, ctx.tokens.md, ctx.tokens.icon);
    let mut body = rect;
    body.take_top(sm);
    let head = body.take_top(ctx.tokens.line);
    body.take_top(xs);
    let under = body.take_top(head.h);

    let mut room = head.pad(Edges::across(md, md));
    waited(ctx, app, &mut room, id);
    let mark = square(room.take_left(size), size);
    room.take_left(sm);
    icon(ctx, mark, Mark::of_kind(&open.session.kind), Role::Faint);
    Label::new(&open.session.title, ctx.styles.label(Role::Text)).draw(ctx, room);

    match asked(app, id) {
        Some((ask, waiting)) => asking(ctx, under, &ask, waiting),
        None => state(ctx, app, under, open),
    }
    hairline(ctx, rect, ctx.styles.line());
    if ui.surface == Surface::Session && app.session.selected.as_ref() == Some(id) {
        ruled(ctx, rect, ctx.styles.chosen());
    }
}

/// The first write the session waits on, and how many wait in all.
fn asked(app: &AppState, id: &SessionId) -> Option<(Ask, usize)> {
    let asks = &app.agent.activity(id)?.asks;
    Some((asks.first()?.clone(), asks.len()))
}

/// The write in peach where the state stands, and its two answers on the line under it.
fn asking(ctx: &mut Ctx, line: Rect, ask: &Ask, waiting: usize) {
    let (md, ground) = (ctx.tokens.md, ctx.styles.ground());
    let room = line.pad(Edges::across(after_mark(ctx, md), md));
    let verb = groove_controllers::agent_service::tools::verb(&ask.op);
    let label = match waiting {
        1 => format!("asks to {verb}"),
        n => format!("asks to {verb} · +{}", n - 1),
    };
    Label::new(&label, ctx.styles.small(Role::Attention)).draw(ctx, room);
    let mut under = Rect::new(room.x, line.bottom(), line.right() - room.x, ctx.tokens.row);
    let approve = Button::new(
        "Approve",
        Target::Approve(ask.id.clone()),
        Role::Attention,
        ground,
    );
    approve.left(ctx, &mut under, ctx.tokens.xs);
    let review = Button::new(
        "Review",
        Target::Examine(ask.id.clone()),
        Role::Muted,
        ground,
    );
    review.left(ctx, &mut under, ctx.tokens.xs);
}

/// How long the agent has waited, at the right of `room`.
fn waited(ctx: &mut Ctx, app: &AppState, room: &mut Rect, id: &SessionId) {
    let sm = ctx.tokens.sm;
    let Some(activity) = app.agent.activity(id) else {
        room.take_right(sm);
        return;
    };
    let text = ago(activity.changed_at.age_at(ctx.now));
    Label::new(&text, ctx.styles.small(Role::Ghost)).right(ctx, room, sm);
}

/// The state line. A working agent's mark turns; every other state stands still.
fn state(ctx: &mut Ctx, app: &AppState, rect: Rect, open: &Open) {
    let (label, role) = state_of(app, open);
    let style = ctx.styles.small(role);
    let mut room = rect.pad(Edges::across(after_mark(ctx, ctx.tokens.md), 0.0));
    if role == Role::Working {
        let box_ = square(room.take_left(style.size), style.size);
        room.take_left(ctx.tokens.xs);
        ctx.icon(box_, Mark::Busy, turn(ctx.tick), style.color);
    }
    Label::new(&label, style).draw(ctx, room);
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
