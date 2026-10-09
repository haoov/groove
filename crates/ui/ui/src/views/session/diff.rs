//! The diff tab: the open file as rows, its two gutters and its colours.

mod blame;
mod finder;
mod header;
mod map;
mod notes;
mod offers;
pub(crate) mod painted;
mod pinned;
mod row;
mod scroll;
mod surface;
mod words;
mod wrap;

use groove_controllers::AppState;
use groove_controllers::workspace_service::At;
use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::components::first;
use crate::ctx::Ctx;
use crate::views::session::Face;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in, row_in};
use groove_ui_kit::text::Label;

pub use blame::Spot;
pub(crate) use blame::{rested, spot};
pub(crate) use map::total as rows_of;
pub(crate) use notes::Inline;
pub(crate) use row::{edited, line_at, text_at};
pub(crate) use scroll::scrolled;

pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    if app.workspace.changes.is_empty() && ui.session.face() != Face::File {
        return hint(ctx, app, area);
    }
    let [head, body] = column_in(area, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    header::draw(ctx, head, app, ui, &standing(ctx, app, ui));
    let [body, column] = row_in(body, [Spec::fill(), Spec::default().width(ctx.tokens.map)]);
    let cols = surface::note_cols(ctx, body, app, ui);
    ctx.app.hits.wraps(cols);
    let inline = Inline::of(app, ui, ui.session.face(), cols);
    map::draw(ctx, (column, body), app, ui, &inline);
    match whole_file(app, ui) {
        Some(true) => said(ctx, body, "Too long to show."),
        Some(false) => said(ctx, body, "Open a file in the sidebar."),
        None => {
            surface::rows(ctx, body, app, ui, &inline);
            let numbers = surface::numbers(app, ui, ui.session.face());
            pinned::draw(ctx, body, app, ui, (&inline, numbers));
            finder::draw(ctx, body, ui);
        }
    }
}

/// What the file view has to say instead of rows: nothing open, or too long to show.
fn whole_file(app: &AppState, ui: &Ui) -> Option<bool> {
    if ui.session.face() != Face::File {
        return None;
    }
    match app.workspace.active() {
        Some(file) => file.long.then_some(true),
        None => Some(false),
    }
}

/// The file the header names: the active one in Files, the one under the top row in the diff.
fn standing(ctx: &Ctx, app: &AppState, ui: &Ui) -> String {
    if ui.session.face() == Face::File {
        let open = app.workspace.active();
        return open.map(|one| one.path.clone()).unwrap_or_default();
    }
    let top = first(ctx.tokens.line, ui.session.diff);
    match app.workspace.changes.at(top) {
        Some(At::Head(file) | At::Row(file, _)) => file.path.clone(),
        None => String::new(),
    }
}

fn said(ctx: &mut Ctx, area: Rect, text: &str) {
    let room = area.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let [line, _] = column_in(room, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    Label::new(text, ctx.styles.body(Role::Faint)).draw(ctx, line);
}

/// What the tab says with nothing to show.
fn hint(ctx: &mut Ctx, app: &AppState, area: Rect) {
    let files = super::files::changed(app).len();
    let text = match files {
        0 => "No change in this worktree.".to_string(),
        _ => "Reading the change\u{2026}".to_string(),
    };
    said(ctx, area, &text);
}
