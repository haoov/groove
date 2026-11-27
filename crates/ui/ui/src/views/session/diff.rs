//! The diff tab: the open file as rows, its two gutters and its colours.

mod blame;
pub(crate) mod blocks;
mod cards;
pub(crate) mod completion;
pub(crate) mod faults;
pub(crate) mod finder;
mod header;
mod map;
pub(crate) mod moded;
mod notes;
mod offers;
pub(crate) mod painted;
mod pinned;
pub(crate) mod popup;
mod row;
mod scroll;
mod surface;
mod words;
pub(crate) mod wrap;
pub(crate) mod wrapping;

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};

use crate::Ui;
use crate::ctx::Ctx;
use crate::views::session::Face;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::layout::{Spec, column_in};
use groove_ui_kit::text::Label;

pub use blame::Spot;
pub(crate) use blame::{rested, spot};
pub(crate) use map::total as rows_of;
pub(crate) use notes::{Inline, noting_drawn};
pub use popup::Pointed;
pub(crate) use row::{edited, is_read, line_at, text_at};
pub(crate) use scroll::scrolled;

/// The surface's rows with their numbers, the scopes held above them, and the find bar.
fn rows(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui, inline: &Inline) {
    surface::rows(ctx, body, app, ui, inline);
    let numbers = surface::numbers(app, ui, ui.session.face());
    pinned::draw(ctx, body, app, ui, (inline, numbers));
}

/// The buffer the editor shows, whole, in `body`: what an object's YAML tab draws.
pub(crate) fn editor(ctx: &mut Ctx, app: &AppState, ui: &Ui, body: Rect) {
    let cols = surface::note_cols(ctx, body, app, ui);
    ctx.app.hits.wraps(cols);
    let inline = Inline::of(app, ui, ui.session.face(), cols);
    rows(ctx, body, app, ui, &inline);
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

fn said(ctx: &mut Ctx, area: Rect, text: &str) {
    let room = area.pad(Edges::across(ctx.tokens.md, ctx.tokens.md));
    let [line, _] = column_in(room, [Spec::default().height(ctx.tokens.row), Spec::fill()]);
    Label::new(text, ctx.styles.body(Role::Faint)).draw(ctx, line);
}

/// What the middle says with nothing to show.
fn hint(ctx: &mut Ctx, app: &AppState, area: Rect) {
    let files = super::files::changed(app).len();
    let text = match files {
        0 => "No change in this worktree.".to_string(),
        _ => "Reading the change\u{2026}".to_string(),
    };
    said(ctx, area, &text);
}
