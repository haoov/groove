//! The session surface: the agent pane, and the workspace's strip with its header and tabs.

pub mod agent_pane;
mod aside;
pub mod diff;
mod face;
pub(crate) mod files;
pub mod find;
pub mod header;
mod manual;
mod mode;
mod moded;
mod open_files;
pub mod overview;
pub mod resources;
mod review;
pub(crate) mod sheet;
mod state;
mod tab;

pub use state::{
    Asked, Bar, Face, Mode, Naming, Noting, Pane, Prompt, SessionUi, Showing, Tab, Term,
};

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
use groove_ui_kit::widgets::{Button, ground_under, solid_pane};

/// The session: the agent on its ground, the workspace in a strip of glass beside it.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    ground_under(ctx, ctx.app.layout.board, Ground::Deep);
    groove_ui_kit::widgets::ground_lattice(ctx, ctx.app.layout.board);
    if crate::render::fires(app, ui) {
        groove_ui_kit::widgets::neurons(ctx, ctx.app.layout.board);
    }
    if !ctx.app.layout.work.is_empty() {
        solid_pane(ctx, ctx.app.layout.strip);
    }
    if app.session.selected().is_none() {
        header::draw(ctx, app, ui);
        empty(ctx, app);
        return crate::views::focus::draw(ctx, app, ui);
    }
    if let Some(open) = app.session.selected().filter(|_| ui.modes) {
        moded::draw(ctx, app, ui, open);
        crate::views::focus::draw(ctx, app, ui);
        return sheet::draw(ctx, app, ui);
    }
    if !ctx.app.layout.workspace.is_empty() {
        header::info(ctx, app, ui);
        workspace(ctx, app, ui);
        sidebar(ctx, app, ui);
    }
    // After the agent: the header, folded or not, and the drawer float over its screen.
    agent_pane::draw(ctx, app, ui);
    crate::views::alerts::draw(ctx, app, ui);
    header::draw(ctx, app, ui);
    if let Some(session) = app.session.selected.as_ref() {
        manual::draw(ctx, app, ui, session);
    }
    crate::views::focus::draw(ctx, app, ui);
    sheet::draw(ctx, app, ui);
}

/// The tab, and the sidebar when it has one.
fn workspace(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    // As far off the strip's bottom edge as the first card stands under the info line's rule.
    let body = ctx.app.layout.workspace.pad(Edges {
        bottom: ctx.tokens.sm,
        ..Edges::all(0.0)
    });
    let band = Spec::default().height(ctx.tokens.row + ctx.tokens.sm);
    match ui.session.tab {
        Tab::Overview => overview::draw(ctx, app, ui, body),
        Tab::Diff if ui.session.reviewing => review::draw(ctx, app, ui, body),
        Tab::Diff => diff::draw(ctx, app, ui, body),
        Tab::Files => {
            let [strip, body] = column_in(body, [band, Spec::fill()]);
            open_files::draw(ctx, strip, app, ui);
            diff::draw(ctx, app, ui, body)
        }
        Tab::Resources => resources::draw(ctx, app, ui, body),
    }
}

/// The sidebar the tab has, if any.
fn sidebar(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
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
