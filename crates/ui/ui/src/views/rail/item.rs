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
use groove_ui_kit::shape::{after_mark, from_left, from_right, from_top, square};
use groove_ui_kit::text::{Label, ago};
use groove_ui_kit::widgets::{dot, glass_button, inset_row, lead};

/// The row's two lines: head and state.
pub fn height(ctx: &Ctx) -> f32 {
    (ctx.tokens.sm + ctx.tokens.line) * 2.0 + ctx.tokens.xs
}

/// State dot, type icon, title and how long it has waited, then the agent's state under them.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect, open: &Open) {
    let id = &open.session.id;
    let up = ui.surface == Surface::Session && app.session.selected.as_ref() == Some(id);
    let tint = tinted(app, id).map(Ground::RowTint);
    let row = inset_row(ctx, rect, Target::Session(id.clone()), (up, tint));
    let (xs, sm, md, _size) = (ctx.tokens.xs, ctx.tokens.sm, ctx.tokens.md, ctx.tokens.icon);
    let mut body = row;
    from_top(&mut body, 0.0, sm);
    let head = from_top(&mut body, ctx.tokens.line, xs);
    let mut under = from_top(&mut body, head.h, 0.0);

    let mut room = head.pad(Edges::across(md, md));
    waited(ctx, app, &mut room, id);
    let (role, hollow) = dot_of(app, id);
    let spot = from_left(&mut room, sm, sm);
    dot(ctx, spot, role, hollow);
    lead(
        ctx,
        &mut room,
        Mark::of_kind(&open.session.kind),
        Role::Faint,
    );
    Label::new(&open.session.title, ctx.styles.label(Role::Text)).draw(ctx, room);

    if let Some(routine) = open.session.kind.routine() {
        run(ctx, app, &mut under, id, routine);
    }
    match asked(app, id) {
        Some((ask, waiting)) => asking(ctx, under, &ask, waiting),
        None => state(ctx, app, under, open),
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

/// Where the state line starts: under the title, past the dot and the type icon.
fn indent(ctx: &Ctx) -> f32 {
    after_mark(ctx, ctx.tokens.md + ctx.tokens.sm * 2.0)
}

/// The write in peach where the state stands.
fn asking(ctx: &mut Ctx, line: Rect, ask: &Ask, waiting: usize) {
    let room = line.pad(Edges::across(indent(ctx), ctx.tokens.md));
    let label = match waiting {
        1 => asks_to(&ask.op),
        n => format!("{} · +{}", asks_to(&ask.op), n - 1),
    };
    Label::new(&label, ctx.styles.small(Role::Attention)).draw(ctx, room);
}

/// The dot's colour, and whether only its edge is drawn.
fn dot_of(app: &AppState, id: &SessionId) -> (Role, bool) {
    let Some(activity) = app.agent.activity(id) else {
        return (Role::Ghost, false);
    };
    if !activity.asks.is_empty() {
        return (Role::Attention, false);
    }
    match activity.status {
        AgentStatus::Asking => (Role::Attention, false),
        AgentStatus::Working => (Role::Working, false),
        AgentStatus::Done { seen: false } => (Role::Ok, false),
        AgentStatus::Error { .. } => (Role::Bad, false),
        AgentStatus::Exited { .. } => (Role::Ghost, true),
        AgentStatus::Idle | AgentStatus::Done { seen: true } => (Role::Ghost, false),
    }
}

/// A routine session's button that runs its routine now, while no run is on it, from the right of `line`.
fn run(ctx: &mut Ctx, app: &AppState, line: &mut Rect, id: &SessionId, routine: &str) {
    if app.routine.runs.busy(id) {
        return;
    }
    let (md, size) = (ctx.tokens.md, ctx.tokens.row);
    from_right(line, 0.0, md);
    let round = square(from_right(line, size, ctx.tokens.sm), size);
    let target = Target::RoutineRun(routine.to_string());
    glass_button(ctx, round, (Mark::Play, Role::Text), target);
}

/// How long the agent has waited, at the right of `room`.
fn waited(ctx: &mut Ctx, app: &AppState, room: &mut Rect, id: &SessionId) {
    let sm = ctx.tokens.sm;
    let Some(activity) = app.agent.activity(id) else {
        from_right(room, 0.0, sm);
        return;
    };
    let text = ago(activity.changed_at.age_at(ctx.now));
    Label::new(&text, ctx.styles.small(Role::Ghost)).right(ctx, room, sm);
}

/// The state line. A working agent's mark turns; every other state stands still.
fn state(ctx: &mut Ctx, app: &AppState, rect: Rect, open: &Open) {
    let (label, role) = state_of(app, open);
    let style = ctx.styles.small(role);
    let mut room = rect.pad(Edges::across(indent(ctx), 0.0));
    if role == Role::Working {
        let box_ = square(from_left(&mut room, style.size, ctx.tokens.xs), style.size);
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
