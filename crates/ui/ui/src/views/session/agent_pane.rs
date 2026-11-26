//! The agent pane: its transcript over Groove's own prompt.

pub(crate) mod aside;
mod asked;
mod chat;
mod dock;
mod edits;
mod entry;
pub(crate) mod model;
pub(crate) mod offered;
mod parts;
mod state;
pub(crate) mod steps;
mod subagent;

use groove_controllers::AppState;
use groove_controllers::agent_service::Agent;
use groove_controllers::session_service::Open;
use groove_types::{AgentStatus, Worktree};

pub use state::AgentUi;

use crate::Ui;
use groove_gfx::{Align, Edges, Rect};
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::row;

/// The agent's conversation, or why there is none.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let pane = ctx.app.layout.agent;
    ctx.hit(pane, Target::Agent);

    let Some(agent) = app.agent.agent(&open.session.id) else {
        return note(ctx, "starting the agent…");
    };
    match (&agent.stream, &agent.activity.status) {
        (Some(_), _) => conversation(ctx, (app, ui), (open, agent)),
        (None, AgentStatus::Error { message }) => note(ctx, message),
        (None, _) => note(ctx, "starting the agent…"),
    }
}

/// The conversation, its usage line at the foot, and over it what the agent asks and the agents
/// it sent that still work.
fn conversation(ctx: &mut Ctx, (app, ui): (&AppState, &Ui), (open, agent): (&Open, &Agent)) {
    let chat = app.agent.chat(&open.session.id);
    let foot = foot(ctx, ui.modes);
    let working = agent.activity.status == AgentStatus::Working;
    let first = agent.permissions.first();
    let asked = first.map(|one| (one, ui.agent.picked(&one.id).len()));
    let place = placed(open);
    let over = asked::ends(ctx, asked, foot);
    let docked = match ui.modes {
        true => Vec::new(),
        false => subagent::docked(chat),
    };
    let dock = dock::laid(ctx, &docked, ui.agent.docked, over, foot);
    let ends = dock.map_or(over, |one| one.y);
    let rows = docked.iter().filter(|_| !ui.agent.docked);
    let wanted: Vec<&str> = rows.map(|one| one.id.as_str()).collect();
    chat::draw(ctx, (open, chat, working), ui, (ends, &wanted));
    ctx.layer();
    if let Some((one, picked)) = asked {
        asked::draw(ctx, one, picked, (foot, &place));
    }
    if let Some(at) = dock {
        dock::draw(ctx, at, &docked, ui.agent.docked);
    }
    if !ui.modes {
        model::line(ctx, chat, agent, foot);
    }
}

/// The usage line at the pane's foot, centred and no wider than the palette; none with modes.
fn foot(ctx: &Ctx, modes: bool) -> Rect {
    let (sm, xs) = (ctx.tokens.sm, ctx.tokens.xs);
    let room = ctx
        .app
        .layout
        .agent
        .pad(Edges::across(sm + xs, sm + xs));
    let wide = room.w.min(ctx.tokens.palette);
    let room = room.align(
        (wide, room.h),
        Align::Center,
        Align::Start,
    );
    let tall = if modes { 0.0 } else { ctx.tokens.row };
    groove_ui_kit::shape::band(room, room.h - tall - xs, tall)
}

/// `groove · feat/x`: the session's first worktree, where an ask acts.
fn placed(open: &Open) -> String {
    let first = open.worktrees.first();
    first.map_or_else(String::new, |one| format!("{} · {}", one.repo, one.branch))
}

fn note(ctx: &mut Ctx, text: &str) {
    let style = ctx.styles.body(Role::Faint);
    let (pad, pane) = (ctx.tokens.md, ctx.app.layout.agent);
    let rect = groove_ui_kit::shape::band(pane, ctx.tokens.sm, ctx.tokens.row);
    row(ctx, rect, pad, text, style);
}

/// The session's worktree that holds `path`, and the path inside it.
pub(crate) fn within<'a>(open: &'a Open, path: &str) -> Option<(&'a Worktree, String)> {
    open.worktrees
        .iter()
        .filter_map(|one| {
            let rest = path.strip_prefix(one.path.trim_end_matches('/'))?;
            Some((one, rest.strip_prefix('/')?.to_string()))
        })
        .max_by_key(|(one, _)| one.path.len())
}
