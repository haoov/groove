//! The session surface: the agent pane, and the workspace's strip with its header and tabs.

pub mod agent_pane;
pub mod bar;
pub mod diff;
mod face;
pub(crate) mod files;
pub mod find;
pub mod header;
mod manual;
mod open_files;
pub mod overview;
pub mod resources;
mod sheet;
mod state;
mod tab;

pub use state::{Asked, Bar, Face, Naming, Noting, Pane, SessionUi, Showing, Tab, Term, Writing};

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::Target;
use crate::keymap::{Action, Keymap};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::{Button, glass_pane, ground_under};

/// The session: the agent on its ground, the workspace in a strip of glass beside it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    ground_under(ctx, ctx.app.layout.board, Ground::Deep);
    if ui.session.alone && app.session.selected().is_some() {
        agent_pane::draw(ctx, app, ui.focus == crate::Focus::Agent);
        return bar::draw(ctx, app);
    }
    if !ctx.app.layout.work.is_empty() {
        glass_pane(ctx, ctx.app.layout.strip);
    }
    header::draw(ctx, app, ui);
    if app.session.selected().is_none() {
        return empty(ctx, app);
    }
    agent_pane::draw(ctx, app, ui.focus == crate::Focus::Agent);
    bar::draw(ctx, app);
    if !ctx.app.layout.workspace.is_empty() {
        header::info(ctx, app, ui);
        workspace(ctx, app, ui);
    }
    if let Some(session) = app.session.selected.as_ref() {
        manual::draw(ctx, app, ui, session);
    }
    sheet::draw(ctx, app, ui);
}

/// The tab, and the sidebar when it has one.
fn workspace(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    let body = ctx.app.layout.workspace;
    let band = Spec::default().height(ctx.tokens.row + ctx.tokens.sm);
    match ui.session.tab {
        Tab::Overview => overview::draw(ctx, app, ui, body),
        Tab::Diff => diff::draw(ctx, app, ui, body),
        Tab::Files => {
            let [strip, body] = column_in(body, [band, Spec::fill()]);
            open_files::draw(ctx, strip, app, ui);
            diff::draw(ctx, app, ui, body)
        }
        Tab::Resources => {
            let [line, body] = column_in(body, [band, Spec::fill()]);
            if let Some(open) = app.session.selected() {
                header::scope(ctx, line, app, open, &ui.session.resources);
            }
            resources::draw(ctx, app, ui, body)
        }
    }
    match ui.session.tab {
        Tab::Resources => resources::sidebar(ctx, app, ui),
        _ if ui.session.sidebar() => files::draw(ctx, app, ui),
        _ => {}
    }
}

/// What folds the sidebar away, at the far end of `strip`. Returns the room left of it.
pub(crate) fn fold(ctx: &mut Ctx, strip: Rect, ui: &Ui) -> Rect {
    let role = match ui.session.folded {
        true => Role::Ghost,
        false => Role::Muted,
    };
    let mut room = strip.pad(Edges::across(0.0, ctx.tokens.sm));
    let fold = Button::icon(Mark::Sidebar, 0, Target::Fold, role);
    fold.right(ctx, &mut room, 0.0);
    room
}

/// Nothing open: how to start.
fn empty(ctx: &mut Ctx, app: &AppState) {
    let (md, tokens) = (ctx.tokens.md, ctx.tokens);
    let bands = [
        Spec::default().height(tokens.sm),
        Spec::default().height(tokens.row),
        Spec::fill(),
    ];
    let [_, line, _] = column_in(ctx.app.layout.workspace, bands);
    let line = line.pad(Edges::across(md, md));
    let keymap = Keymap::of(app.config.config.as_ref());
    let said = match keymap.label(Action::NewExplorer) {
        Some(chord) => format!("No session open. {chord} starts an explorer."),
        None => "No session open. The palette starts an explorer.".to_string(),
    };
    Label::new(&said, ctx.styles.body(Role::Faint)).draw(ctx, line);
}
