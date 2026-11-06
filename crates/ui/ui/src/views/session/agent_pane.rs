//! The agent pane: its conversation over Groove's own prompt.

mod asked;
mod chat;
mod commands;
mod dock;
mod entry;
mod glance;
pub(crate) mod model;
pub(crate) mod offered;
mod prompt;
mod subagent;

use groove_controllers::AppState;
use groove_controllers::agent_service::Agent;
use groove_controllers::session_service::Open;
use groove_types::{AgentStatus, Worktree};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::row;

/// The agent's conversation, or why there is none.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, focused: bool) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let pane = ctx.app.layout.agent;
    ctx.hit(pane, Target::Agent);

    let Some(agent) = app.agent.agent(&open.session.id) else {
        return note(ctx, "starting the agent…");
    };
    match (&agent.stream, &agent.activity.status) {
        (Some(_), _) => conversation(ctx, (app, ui), (open, agent), focused),
        (None, AgentStatus::Error { message }) => note(ctx, message),
        (None, _) => note(ctx, "starting the agent…"),
    }
}

/// The conversation, and over the prompt what the agent asks and the agents it sent that still work.
fn conversation(
    ctx: &mut Ctx,
    (app, ui): (&AppState, &Ui),
    (open, agent): (&Open, &Agent),
    focused: bool,
) {
    let chat = app.agent.chat(&open.session.id);
    let laid = prompt::laid(ctx, ui);
    let working = agent.activity.status == AgentStatus::Working;
    let first = agent.permissions.first();
    let asked = first.map(|one| (one, ui.agent.picked(&one.id).len()));
    let over = asked::ends(ctx, asked, laid.pane);
    let docked = subagent::docked(chat);
    let dock = dock::laid(ctx, &docked, ui.agent.docked, over, laid.pane);
    let ends = dock.map_or(over, |one| one.y);
    let rows = docked.iter().filter(|_| !ui.agent.docked);
    let wanted: Vec<&str> = rows.map(|one| one.id.as_str()).collect();
    chat::draw(ctx, (open, chat, working), ui, (ends, &wanted));
    ctx.layer();
    if let Some((one, picked)) = asked {
        asked::draw(ctx, one, picked, laid.pane);
    }
    if let Some(at) = dock {
        dock::draw(ctx, at, &docked, ui.agent.docked);
    }
    prompt::draw(ctx, ui, &laid, focused);
    prompt::offers(ctx, &offered::offered(app, ui), ui.agent.offered, laid.pane);
    model::draw(ctx, chat, agent, laid.pane);
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
