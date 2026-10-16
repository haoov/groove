//! How many characters a row holds this frame: a note's, and a line of code's while it wraps.

use groove_controllers::AppState;
use groove_gfx::{Edges, Rect};
use groove_types::DiffView;

use super::{chars_of, halves, numbers};
use crate::Ui;
use crate::ctx::Ctx;
use crate::views::session::Face;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::base::tokens::{NOTE_BY, NOTE_SLACK};
use groove_ui_kit::layout::{Boxes, Spec};

/// The characters a row of the file view holds, kept for the wrap to read; the view drawn.
pub(super) fn code_cols(ctx: &mut Ctx, (app, ui): (&AppState, &Ui), rect: Rect) -> Face {
    let view = ui.session.face();
    if view == Face::File {
        let chars = chars_of(ctx, numbers(app, ui, view), rect, (0.0, 0.0));
        let room = rect.right() - chars.left - ctx.tokens.md;
        ui.wrapping
            .measured((room / chars.advance).floor().max(0.0) as usize);
    }
    view
}

/// How many characters a note row holds this frame, in the surface that draws the notes.
pub(in crate::views::session::diff) fn note_cols(
    ctx: &mut Ctx,
    body: Rect,
    app: &AppState,
    ui: &Ui,
) -> usize {
    let view = ui.session.face();
    let rect = match view {
        Face::Stream(DiffView::Split) => halves(ctx, body).1,
        _ => body,
    };
    let sample = "the quick brown fox jumps over the lazy dog";
    let words = ctx.styles.body(Role::Text);
    let each = ctx.measure(sample, &words) / sample.len() as f32 * NOTE_SLACK;
    let text = chars_of(ctx, numbers(app, ui, view), rect, (0.0, 0.0)).left;
    let mut boxes = Boxes::new();
    let note = boxes.leaf(Spec::default().grow(1.0));
    let by = boxes.leaf(Spec::default().width(each * NOTE_BY));
    let pad = Edges {
        left: text - rect.x,
        right: ctx.tokens.md,
        ..Edges::default()
    };
    let row = boxes.row(Spec::default().pad(pad), &[note, by]);
    boxes.solve(ctx, row, rect);
    (boxes.rect(note).w / each).floor().max(0.0) as usize
}
