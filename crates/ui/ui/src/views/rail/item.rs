use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{AgentStatus, Ask, AttentionClass, SessionId};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::{Surface, Ui};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::{after_mark, hairline, ruled, square};
use groove_ui_kit::text::{Label, ago};
use groove_ui_kit::widgets::{Button, lead};

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
    let up = ui.surface == Surface::Session && app.session.selected.as_ref() == Some(id);
    if up {
        groove_ui_kit::shape::ground(ctx, rect, Ground::Held);
    }
    groove_ui_kit::shape::hoverable(ctx, rect, Target::Session(id.clone()));
    if let Some(role) = tinted(app, id) {
        groove_ui_kit::shape::ground(ctx, rect, Ground::Tint(role));
    }
    let (xs, sm, md, _size) = (ctx.tokens.xs, ctx.tokens.sm, ctx.tokens.md, ctx.tokens.icon);
    let mut body = rect;
    body.take_top(sm);
    let head = body.take_top(ctx.tokens.line);
    body.take_top(xs);
    let under = body.take_top(head.h);

    let mut room = head.pad(Edges::across(md, md));
    waited(ctx, app, &mut room, id);
    lead(
        ctx,
        &mut room,
        Mark::of_kind(&open.session.kind),
        Role::Faint,
    );
    Label::new(&open.session.title, ctx.styles.label(Role::Text)).draw(ctx, room);

    match asked(app, id) {
        Some((ask, waiting)) => asking(ctx, under, &ask, waiting),
        None => state(ctx, app, under, open),
    }
    if let Some(routine) = open.session.kind.routine() {
        run(ctx, app, under, id, routine);
    }
    hairline(ctx, rect, ctx.styles.line());
    if up {
        ruled(ctx, rect, ctx.styles.chosen());
    }
}

/// The first write the session waits on, and how many wait in all.
fn asked(app: &AppState, id: &SessionId) -> Option<(Ask, usize)> {
    let asks = &app.agent.activity(id)?.asks;
    Some((asks.first()?.clone(), asks.len()))
}

fn tinted(app: &AppState, id: &SessionId) -> Option<Role> {
    let activity = app.agent.activity(id)?;
    if !activity.asks.is_empty() {
        return Some(Role::Attention);
    }
    match activity.status {
        AgentStatus::Asking => Some(Role::Attention),
        AgentStatus::Done { seen: false } => Some(Role::Ok),
        _ => None,
    }
}

/// `asks to commit`: what a write does, as the rail and the sheet say it.
pub(crate) fn asks_to(op: &str) -> String {
    format!(
        "asks to {}",
        groove_controllers::agent_service::tools::verb(op)
    )
}

/// The write in peach where the state stands, and its two answers on the line under it.
fn asking(ctx: &mut Ctx, line: Rect, ask: &Ask, waiting: usize) {
    let (md, ground, hover) = (ctx.tokens.md, ctx.styles.ground(), ctx.styles.hover());
    let room = line.pad(Edges::across(after_mark(ctx, md), md));
    let label = match waiting {
        1 => asks_to(&ask.op),
        n => format!("{} · +{}", asks_to(&ask.op), n - 1),
    };
    Label::new(&label, ctx.styles.small(Role::Attention)).draw(ctx, room);
    let mut under = Rect::new(room.x, line.bottom(), line.right() - room.x, ctx.tokens.row);
    for (label, target) in [
        ("Approve", Target::Approve(ask.id.clone())),
        ("Review", Target::Examine(ask.id.clone())),
    ] {
        let button = Button::new(label, target, Role::Text, ground).hover(hover);
        button.left(ctx, &mut under, ctx.tokens.xs);
    }
}

/// A routine session's button that runs its routine now, while no run is on it.
fn run(ctx: &mut Ctx, app: &AppState, line: Rect, id: &SessionId, routine: &str) {
    if app.agent.runs.busy(id) {
        return;
    }
    let (md, ground, hover) = (ctx.tokens.md, ctx.styles.ground(), ctx.styles.hover());
    let mut room = line.pad(Edges::across(0.0, md));
    let target = Target::RoutineRun(routine.to_string());
    let button = Button::new("Run", target, Role::Text, ground).hover(hover);
    button.right(ctx, &mut room, 0.0);
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
        groove_ui_kit::widgets::busy(ctx, box_, role);
    }
    Label::new(&label, style).draw(ctx, room);
}

/// What the agent is doing, and its colour.
fn state_of(app: &AppState, open: &Open) -> (String, Role) {
    let Some(activity) = app.agent.activity(&open.session.id) else {
        return ("idle".into(), Role::Ghost);
    };
    let label = match &activity.status {
        AgentStatus::Working => "working".into(),
        AgentStatus::Asking => "asks you".into(),
        AgentStatus::Done { .. } => "done".into(),
        AgentStatus::Idle => "idle".into(),
        AgentStatus::Exited { code } => format!("exited {code}"),
        AgentStatus::Error { message } => format!("error: {message}"),
    };
    let role = match activity.class() {
        AttentionClass::NeedsYou => Role::Attention,
        AttentionClass::Moving => Role::Working,
        AttentionClass::ActWhenYouLook => Role::Ok,
        AttentionClass::Quiet => Role::Ghost,
    };
    (label, role)
}
