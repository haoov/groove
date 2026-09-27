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
use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::base::ctx::Ctx;
use crate::base::hit::Target;
use crate::base::mark::Mark;
use crate::base::style::Role;
use crate::shape::square;
use crate::text::Label;
use crate::widgets::{icon, tabs};

/// The session: the header, then the agent pane and the workspace.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    header::draw(ctx, app);
    if app.session.selected().is_none() {
        return empty(ctx);
    }
    agent_pane::draw(ctx, app);
    bar::draw(ctx, app);
    workspace(ctx, app, ui);
    sheet::draw(ctx, app, ui);
}

/// The workspace: the tab strip, then the tab.
fn workspace(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let mut body = ctx.layout.workspace;
    let strip = body.take_top(ctx.tokens.row + ctx.tokens.sm);
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

    match ui.session.tab {
        Tab::Overview => {
            body.take_top(ctx.tokens.sm);
            overview::draw(ctx, app, ui, body)
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
    let size = ctx.tokens.icon;
    let mut room = strip.pad(Edges::across(0.0, ctx.tokens.md));
    let box_ = square(room.take_right(size), size);
    icon(ctx, box_, Mark::Sidebar, role);
    ctx.hit(box_, Target::Fold);
}

/// Nothing open: how to start.
fn empty(ctx: &mut Ctx) {
    let (md, mut body) = (ctx.tokens.md, ctx.layout.workspace);
    body.take_top(ctx.tokens.sm);
    let line = body.take_top(ctx.tokens.row).pad(Edges::across(md, md));
    let said = "No session open. Ctrl+Shift+N starts an explorer.";
    Label::new(said, ctx.styles.body(Role::Faint)).draw(ctx, line);
}
