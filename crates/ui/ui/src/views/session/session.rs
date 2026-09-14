use groove_controllers::AppState;
use groove_gfx::Rect;

use super::components::{agent_pane, header, overview};
use crate::Ui;
use crate::ctx::Ctx;
use crate::style::Role;
use crate::widget::{row, tabs};

/// Which tab of the workspace is up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Overview,
}

impl Tab {
    /// Every tab, in the order the strip shows them.
    pub const ALL: [Tab; 1] = [Tab::Overview];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
        }
    }
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
    workspace(ctx, app, ui);
}

/// The workspace: the tab strip, then the tab.
fn workspace(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let area = ctx.layout.workspace;
    let strip = Rect::new(area.x, area.y, area.w, ctx.tokens.row);
    let labels: Vec<&str> = Tab::ALL.iter().map(|tab| tab.label()).collect();
    let at = Tab::ALL
        .iter()
        .position(|tab| *tab == ui.session.tab)
        .unwrap_or(0);
    tabs(ctx, strip, &labels, at);

    let body = Rect::new(
        area.x,
        strip.bottom() + ctx.tokens.sm,
        area.w,
        area.h - ctx.tokens.row,
    );
    match ui.session.tab {
        Tab::Overview => overview::draw(ctx, app, body),
    }
}

/// Nothing open: how to start.
fn empty(ctx: &mut Ctx) {
    let style = ctx.styles.body(Role::Faint);
    let (pad, body) = (ctx.tokens.md, ctx.layout.workspace);
    let rect = Rect::new(body.x, body.y + ctx.tokens.sm, body.w, ctx.tokens.row);
    row(
        ctx,
        rect,
        pad,
        "No session open. Ctrl+Shift+N starts an explorer.",
        style,
    );
}
