//! The session surface: its bar, the mode in the middle, its sidebar and the terminals.

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
pub mod overview;
pub mod resources;
mod review;
pub(crate) mod sheet;
mod state;
mod working;

pub use state::{Asked, Face, Mode, Naming, Noting, Pane, Prompt, SessionUi, Showing};

use groove_controllers::AppState;
use groove_gfx::Edges;

use crate::Ui;
use crate::ctx::Ctx;
use crate::keymap::{Action, Keymap};
use groove_ui_kit::base::ground::Ground;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;
use groove_ui_kit::widgets::ground_under;

/// The session: its bar of title and tabs, the mode in the middle, its sidebar at the right.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui) {
    ground_under(ctx, ctx.app.layout.board, Ground::Deep);
    groove_ui_kit::widgets::ground_lattice(ctx, ctx.app.layout.board);
    if crate::render::fires(app, ui) {
        groove_ui_kit::widgets::neurons(ctx, ctx.app.layout.board);
    }
    let Some(open) = app.session.selected() else {
        empty(ctx, app);
        return crate::views::focus::draw(ctx, app, ui);
    };
    moded::draw(ctx, app, ui, open);
    crate::views::focus::draw(ctx, app, ui);
    sheet::draw(ctx, app, ui);
}

/// Nothing open: how to start.
fn empty(ctx: &mut Ctx, app: &AppState) {
    let (md, tokens) = (ctx.tokens.md, ctx.tokens);
    let bands = [
        Spec::default().height(tokens.sm),
        Spec::default().height(tokens.row),
        Spec::fill(),
    ];
    let [_, line, _] = column_in(ctx.app.layout.agent, bands);
    let line = line.pad(Edges::across(md, md));
    let keymap = Keymap::of(app.config.config.as_ref());
    let said = match keymap.label(Action::NewExplorer) {
        Some(chord) => format!("No session open. {chord} starts an explorer."),
        None => "No session open. The palette starts an explorer.".to_string(),
    };
    Label::new(&said, ctx.styles.body(Role::Faint)).draw(ctx, line);
}
