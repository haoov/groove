use groove_controllers::AppState;
use groove_gfx::Rect;

use super::components::{agent_pane, header, overview};
use crate::Ui;
use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::row;

/// Which tab of the workspace is up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Overview,
}

/// What the session surface remembers between frames.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SessionUi {
    pub tab: Tab,
}

/// The session: one header line, then the agent pane and the workspace.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    header::draw(ctx, app);
    if app.session.selected().is_none() {
        return empty(ctx);
    }
    agent_pane::draw(ctx, app);
    match ui.session.tab {
        Tab::Overview => overview::draw(ctx, app),
    }
}

/// Nothing open: how to start.
fn empty(ctx: &mut Ctx) {
    let style = ctx.styles.body(Role::Muted);
    let (pad, body) = (ctx.tokens.md, ctx.layout.body);
    let rect = Rect::new(body.x, body.y + ctx.tokens.sm, body.w, ctx.tokens.row);
    row(
        ctx,
        rect,
        pad,
        "No session open. Ctrl+Shift+N starts an explorer.",
        style,
    );
}
