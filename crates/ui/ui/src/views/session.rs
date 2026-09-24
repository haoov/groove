//! The session surface: the header, the agent pane, the workspace and its tabs.

pub mod agent_pane;
pub mod bar;
pub mod commit;
pub mod diff;
pub(crate) mod files;
pub mod find;
pub mod header;
pub mod overview;
mod sheet;
mod state;
pub mod worktree_row;

pub(crate) use files::changed;
pub use state::{Asked, Bar, Naming, Noting, Pane, Scope, SessionUi, Tab, Term, Writing};

use groove_controllers::AppState;
use groove_gfx::Rect;

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::mark::Mark;
use crate::style::Role;
use crate::widget::{icon, leading, row, tabs};

/// The session: the header, then the agent pane and the workspace.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    header::draw(ctx, app, ui);
    if app.session.selected().is_none() {
        return empty(ctx);
    }
    agent_pane::draw(ctx, app);
    bar::draw(ctx, app, ui);
    workspace(ctx, app, ui);
    sheet::draw(ctx, app, ui);
}

/// The workspace: the tab strip, then the tab.
fn workspace(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let area = ctx.layout.workspace;
    let strip = Rect::new(area.x, area.y, area.w, ctx.tokens.row + ctx.tokens.sm);
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
            overview::draw(ctx, app, ui, inset)
        }
        Tab::File => diff::draw(ctx, app, ui, body),
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
