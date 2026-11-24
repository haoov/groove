use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_gfx::{Edges, Rect};
use groove_types::{AgentStatus, Ask, AttentionClass, SessionId};

use super::kind;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::{Surface, Ui};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::Tokens;
use groove_ui_kit::shape::{from_left, from_top, ground, square};
use groove_ui_kit::text::{Label, ago};
use groove_ui_kit::widgets::flat_row;

/// The row's two lines: head and state.
pub fn height(tokens: &Tokens) -> f32 {
    (tokens.sm + tokens.line) * 2.0 + tokens.xs
}

pub fn child_height(tokens: &Tokens) -> f32 {
    tokens.row
}

/// Tag, number, title and age, then the agent's state under them.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect, open: &Open) {
    let id = &open.session.id;
    let row = grounded(ctx, app, ui, rect, id);
    let (xs, sm, md) = (ctx.tokens.xs, ctx.tokens.sm, ctx.tokens.md);
    let mut body = row;
    from_top(&mut body, 0.0, sm);
    let head = from_top(&mut body, ctx.tokens.line, xs);
    let under = from_top(&mut body, head.h, 0.0);

    let mut room = head.pad(Edges::across(md, md));
    waited(ctx, app, &mut room, id);
    tagged(ctx, app, &mut room, open);
    titled(ctx, app, room, open);

    let under = under.pad(Edges::across(md + tag_width(ctx), md));
    match asked(app, id) {
        Some((ask, waiting)) => asking(ctx, under, &ask, waiting),
        None => state(ctx, app, under, open),
    }
}

/// A child on one line: branch, tag, title, then its state and age.
pub fn draw_child(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect, open: &Open, last: bool) {
    let id = &open.session.id;
    let row = grounded(ctx, app, ui, rect, id);
    let md = ctx.tokens.md;
    let mut room = row.pad(Edges::across(md + tag_width(ctx), md));
    let (label, role) = match asked(app, id) {
        Some((ask, _)) => (asks_to(&ask.op), Role::Attention),
        None => state_of(app, open),
    };
    let since = activity_age(ctx, app, id);
    let said = since.map_or(label.clone(), |age| format!("{label} · {age}"));
    Label::new(&said, ctx.styles.small(role)).right(ctx, &mut room, ctx.tokens.sm);
    let branch = if last { "└" } else { "├" };
    Label::new(branch, ctx.styles.tag(Role::Ghost)).left(ctx, &mut room, ctx.tokens.sm);
    tagged(ctx, app, &mut room, open);
    titled(ctx, app, room, open);
}

/// The row's tint, then its held or hovered ground over it.
fn grounded(ctx: &mut Ctx, app: &AppState, ui: &Ui, rect: Rect, id: &SessionId) -> Rect {
    if let Some(role) = tint_of(app, id) {
        ground(ctx, rect, Ground::RowTint(role));
    }
    let up = ui.surface == Surface::Session && app.session.selected.as_ref() == Some(id);
    flat_row(ctx, rect, Target::Session(id.clone()), up)
}

/// Attention while it asks, done while done and unseen.
fn tint_of(app: &AppState, id: &SessionId) -> Option<Role> {
    let activity = app.agent.activity(id)?;
    match (&activity.status, activity.asks.is_empty()) {
        (_, false) | (AgentStatus::Asking, _) => Some(Role::Attention),
        (AgentStatus::Done { seen: false }, _) => Some(Role::Ok),
        _ => None,
    }
}

/// The width of the tags' column, gap included.
fn tag_width(ctx: &mut Ctx) -> f32 {
    let style = ctx.styles.tag(Role::Faint);
    Label::new("TSK", style).width(ctx) + ctx.tokens.sm
}

/// The kind tag in the state's colour, in its own column at the left of `room`.
fn tagged(ctx: &mut Ctx, app: &AppState, room: &mut Rect, open: &Open) {
    let column = from_left(room, tag_width(ctx), 0.0);
    let style = ctx.styles.tag(role_of(app, &open.session.id));
    Label::new(kind::tag(app, open), style).draw(ctx, column);
}

fn titled(ctx: &mut Ctx, app: &AppState, mut room: Rect, open: &Open) {
    if let Some(number) = kind::number(app, open) {
        let style = ctx.styles.label(Role::Faint);
        Label::new(&number, style).left(ctx, &mut room, ctx.tokens.sm);
    }
    Label::new(&open.session.title, ctx.styles.label(Role::Text)).draw(ctx, room);
}

/// The first write the session waits on, and how many wait in all.
fn asked(app: &AppState, id: &SessionId) -> Option<(Ask, usize)> {
    let asks = &app.agent.activity(id)?.asks;
    Some((asks.first()?.clone(), asks.len()))
}

/// `asks to commit`: what a write does, as the rail and the sheet say it.
pub(crate) fn asks_to(op: &str) -> String {
    format!(
        "asks to {}",
        groove_controllers::agent_service::tools::verb(op)
    )
}

/// The write in the attention colour where the state stands.
fn asking(ctx: &mut Ctx, line: Rect, ask: &Ask, waiting: usize) {
    let label = match waiting {
        1 => asks_to(&ask.op),
        n => format!("{} · +{}", asks_to(&ask.op), n - 1),
    };
    Label::new(&label, ctx.styles.small(Role::Attention)).draw(ctx, line);
}

/// The tag's colour: the session's state.
fn role_of(app: &AppState, id: &SessionId) -> Role {
    let Some(activity) = app.agent.activity(id) else {
        return Role::Faint;
    };
    if !activity.asks.is_empty() {
        return Role::Attention;
    }
    match activity.status {
        AgentStatus::Asking => Role::Attention,
        AgentStatus::Working => Role::Working,
        AgentStatus::Done { seen: false } => Role::Ok,
        AgentStatus::Error { .. } => Role::Bad,
        AgentStatus::Exited { .. } | AgentStatus::Idle | AgentStatus::Done { seen: true } => {
            Role::Faint
        }
    }
}

fn activity_age(ctx: &Ctx, app: &AppState, id: &SessionId) -> Option<String> {
    let activity = app.agent.activity(id)?;
    Some(ago(activity.changed_at.age_at(ctx.now)))
}

/// How long the agent has waited, at the right of `room`.
fn waited(ctx: &mut Ctx, app: &AppState, room: &mut Rect, id: &SessionId) {
    let sm = ctx.tokens.sm;
    let Some(text) = activity_age(ctx, app, id) else {
        room.take_right(sm);
        return;
    };
    Label::new(&text, ctx.styles.small(Role::Ghost)).right(ctx, room, sm);
}

/// The state line. A working agent's mark turns; every other state stands still.
fn state(ctx: &mut Ctx, app: &AppState, rect: Rect, open: &Open) {
    let (label, role) = state_of(app, open);
    let style = ctx.styles.small(role);
    let mut room = rect;
    if role == Role::Working {
        let box_ = square(from_left(&mut room, style.size, ctx.tokens.xs), style.size);
        groove_ui_kit::widgets::busy(ctx, box_, role);
    }
    Label::new(&label, style).draw(ctx, room);
}

/// What the agent is doing, and its colour.
pub(crate) fn state_of(app: &AppState, open: &Open) -> (String, Role) {
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
