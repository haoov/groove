use groove_controllers::AppState;
use groove_gfx::Rect;
use groove_types::DiffView;

use super::components::{agent_pane, diff, files, header, overview};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{icon, leading, row, tabs};

/// Which tab of the workspace is up.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Overview,
    Diff,
}

impl Tab {
    /// Every tab, in the order the strip shows them.
    pub const ALL: [Tab; 2] = [Tab::Overview, Tab::Diff];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "overview",
            Tab::Diff => "diff",
        }
    }

    /// Whether the tab brings its own list beside the workspace.
    pub fn has_sidebar(self) -> bool {
        match self {
            Tab::Overview => false,
            Tab::Diff => true,
        }
    }
}

/// What the session surface remembers between frames.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SessionUi {
    pub tab: Tab,
    /// How far the sidebar's list is scrolled, in pixels.
    pub files: f32,
    /// The user folded the sidebar away.
    pub folded: bool,
    /// How far the open file is scrolled, in pixels.
    pub diff: f32,
    /// Which of the three views the open file is drawn in.
    pub view: DiffView,
    /// The keyboard is in the commit box.
    pub composing: bool,
}

impl SessionUi {
    /// Whether the sidebar stands beside the workspace right now.
    pub fn sidebar(&self) -> bool {
        self.tab.has_sidebar() && !self.folded
    }
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
    for (tab, rect) in Tab::ALL.iter().zip(tabs(ctx, strip, &labels, at)) {
        ctx.hit(rect, Target::Tab(*tab));
    }
    if ui.session.tab.has_sidebar() {
        fold(ctx, strip, ui);
    }

    let body = Rect::new(area.x, strip.bottom(), area.w, area.h - strip.h);
    match ui.session.tab {
        Tab::Overview => {
            let inset = Rect::new(
                body.x,
                body.y + ctx.tokens.sm,
                body.w,
                body.h - ctx.tokens.sm,
            );
            overview::draw(ctx, app, inset)
        }
        Tab::Diff => diff::draw(ctx, app, ui, body),
    }
    if ui.session.sidebar() {
        files::draw(ctx, app, ui);
    }
}

/// What folds the sidebar away, at the far end of the strip.
fn fold(ctx: &mut Ctx, strip: Rect, ui: &Ui) {
    let role = match ui.session.folded {
        true => Role::Ghost,
        false => Role::Muted,
    };
    let at = strip.right() - ctx.tokens.md - ctx.tokens.icon;
    let box_ = leading(ctx, strip, at);
    icon(ctx, box_, Mark::Sidebar, role);
    ctx.hit(box_, Target::Fold);
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
