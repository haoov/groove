//! The agent pane: its conversation over Groove's own prompt, or its terminal in their place.

mod asked;
mod chat;
mod commands;
mod entry;
mod glance;
mod prompt;

use groove_controllers::AppState;
use groove_controllers::session_service::Open;
use groove_types::{AgentStatus, Worktree};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::row;
use groove_ui_kit::widgets::screen;

/// The agent's conversation or its terminal, or why there is neither.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, focused: bool) {
    let Some(open) = app.session.selected() else {
        return;
    };
    let pane = ctx.app.layout.agent;
    ctx.hit(pane, Target::Agent);

    let Some(agent) = app.agent.agent(&open.session.id) else {
        return note(ctx, "starting the agent…");
    };
    match (&agent.terminal, &agent.activity.status) {
        _ if agent.stream.is_some() || app.config.conversation() && agent.terminal.is_some() => {
            let chat = app.agent.chat(&open.session.id);
            let laid = prompt::laid(ctx, ui);
            let working = agent.activity.status == AgentStatus::Working;
            let first = agent.permissions.first();
            let asked = first.map(|one| (one, ui.agent.picked(&one.id).len()));
            let ends = asked::ends(ctx, asked, laid.pane);
            chat::draw(ctx, (open, chat, working), ui, ends);
            ctx.layer();
            if let Some((one, picked)) = asked {
                asked::draw(ctx, one, picked, laid.pane);
            }
            prompt::draw(ctx, ui, &laid, focused);
        }
        (Some(terminal), _) => {
            let origin = ctx.app.layout.agent_origin(&ctx.tokens);
            let grid = terminal.screen();
            screen(ctx, pane, origin, (&grid, focused));
        }
        (None, AgentStatus::Error { message }) => note(ctx, message),
        (None, _) => note(ctx, "starting the agent…"),
    }
}

/// Whether the selected session's agent shows its conversation: headless, or by the user's choice.
pub(crate) fn conversing(app: &AppState) -> bool {
    let session = app.session.selected.as_ref();
    let headless = session.and_then(|one| app.agent.stream(one)).is_some();
    headless || app.config.conversation()
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
