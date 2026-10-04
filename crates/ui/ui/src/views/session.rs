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
use crate::keymap::{Action, Keymap};
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, tabs};

/// The session: the header, then the agent pane and the workspace.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    if ui.session.alone && app.session.selected().is_some() {
        agent_pane::draw(ctx, app, ui.focus == crate::Focus::Agent);
        return bar::draw(ctx, app);
    }
    header::draw(ctx, app);
    if app.session.selected().is_none() {
        return empty(ctx, app);
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
    let shown: Vec<(&str, Target)> = Tab::ALL
        .iter()
        .map(|tab| (tab.label(), Target::Tab(*tab)))
        .collect();
    let at = Tab::ALL
        .iter()
        .position(|tab| *tab == ui.session.tab)
        .unwrap_or(0);
    tabs(ctx, strip, &shown, at);
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
    let mut room = strip.pad(Edges::across(0.0, ctx.tokens.sm));
    let fold = Button::icon(Mark::Sidebar, 0, Target::Fold, role);
    fold.right(ctx, &mut room, 0.0);
}

/// Nothing open: how to start.
fn empty(ctx: &mut Ctx, app: &AppState) {
    let (md, mut body) = (ctx.tokens.md, ctx.app.layout.workspace);
    body.take_top(ctx.tokens.sm);
    let line = body.take_top(ctx.tokens.row).pad(Edges::across(md, md));
    let keymap = Keymap::of(app.config.config.as_ref());
    let said = match keymap.label(Action::NewExplorer) {
        Some(chord) => format!("No session open. {chord} starts an explorer."),
        None => "No session open. The palette starts an explorer.".to_string(),
    };
    Label::new(&said, ctx.styles.body(Role::Faint)).draw(ctx, line);
}
