//! The Diff tab in the modes layout: one file of the change, its switches on a band over its
//! rows, and a thin map of that file beside them, all on the ground.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_ui_kit::layout::{Spec, column_in, row_in};

use super::Inline;
use crate::Ui;
use crate::ctx::Ctx;

/// The middle in `area`, a gutter as the conversation's at its sides.
pub fn draw(ctx: &mut Ctx, app: &AppState, ui: &Ui, area: Rect) {
    let column = area.pad(Edges::xy(ctx.tokens.xl, ctx.tokens.sm));
    body(ctx, app, ui, column);
}

/// The band and the file's rows in `column`, which the cluster write's band may stand over.
pub fn body(ctx: &mut Ctx, app: &AppState, ui: &Ui, column: Rect) {
    let tall = |height: f32| Spec::default().height(height);
    let [band, _, body] = column_in(
        column,
        [tall(ctx.tokens.row), tall(ctx.tokens.sm), Spec::fill()],
    );
    let mut room = band;
    super::header::switches(ctx, &mut room, app, ui);
    if app.workspace.changes.is_empty() {
        return super::hint(ctx, app, body);
    }
    let wide = |width: f32| Spec::default().width(width);
    let [body, _, map] = row_in(
        body,
        [Spec::fill(), wide(ctx.tokens.sm), wide(ctx.tokens.map_thin)],
    );
    let cols = super::surface::note_cols(ctx, body, app, ui);
    ctx.app.hits.wraps(cols);
    let inline = Inline::of(app, ui, ui.session.face(), cols);
    super::map::draw(ctx, (map, body), app, ui, &inline);
    super::rows(ctx, body, app, ui, &inline);
}
