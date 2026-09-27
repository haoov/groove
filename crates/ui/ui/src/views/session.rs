//! The session surface: the header, the agent pane, the workspace and its tabs.

pub mod agent_pane;
pub mod bar;
pub mod commit;
pub mod diff;
pub(crate) mod files;
pub mod find;
pub mod header;
mod manual;
mod open_files;
pub mod overview;
mod sheet;
mod state;
mod tab;

pub(crate) use files::changed;
pub use state::{Asked, Bar, Face, Naming, Noting, Pane, SessionUi, Tab, Term, Writing};

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::shape::square;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{icon, tabs};

/// The session: the header, then the agent pane and the workspace.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    header::draw(ctx, app);
    if app.session.selected().is_none() {
        return empty(ctx);
    }
    agent_pane::draw(ctx, app, ui.focus == crate::Focus::Agent);
    bar::draw(ctx, app);
    workspace(ctx, app, ui);
    if let Some(session) = app.session.selected.as_ref() {
        manual::draw(ctx, app, ui, session);
    }
    sheet::draw(ctx, app, ui);
}

/// The workspace: the tab strip, then the tab.
fn workspace(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let mut body = ctx.app.layout.workspace;
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
        Tab::Diff => diff::draw(ctx, app, ui, body),
        Tab::Files => {
            let strip = body.take_top(ctx.tokens.row + ctx.tokens.sm);
            open_files::draw(ctx, strip, app, ui);
            diff::draw(ctx, app, ui, body)
        }
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
    let (md, mut body) = (ctx.tokens.md, ctx.app.layout.workspace);
    body.take_top(ctx.tokens.sm);
    let line = body.take_top(ctx.tokens.row).pad(Edges::across(md, md));
    let said = "No session open. Ctrl+Shift+N starts an explorer.";
    Label::new(said, ctx.styles.body(Role::Faint)).draw(ctx, line);
}
