//! A change a write would make, read-only: one aligned file, inline or split, coloured as the diff is.

use std::ops::Range;
use std::rc::Rc;

use groove_controllers::workspace_service::{Colours, Review};
use groove_gfx::Rect;
use groove_types::DiffView;

use super::row::{Drawn, Side, plain, source, stretch};
use super::surface::{gutters_of, halves, lined};
use crate::components::{Gutters, Line, Rows, across_extent, code, height, visible};
use crate::ctx::Ctx;
use crate::hit::Scroller;
use crate::{Examining, Ui};

/// The change `review` holds, laid out and scrolled as `sheet` says; `key` keeps its colours apart.
pub(crate) fn draw(ctx: &mut Ctx, ui: &Ui, body: Rect, (review, sheet): (&Review, &Examining)) {
    let key = format!("{}#{}", review.aligned.path, sheet.id.as_str());
    let shown = Shown {
        review,
        key: &key,
        view: sheet.view,
        at: (sheet.scroll, sheet.across),
    };
    match sheet.view {
        DiffView::Split => {
            let (left, right) = halves(ctx, body);
            let rule = ctx.styles.line();
            groove_ui_kit::shape::side_rule(ctx, body, left.right(), rule);
            side(ctx, ui, left, &shown, Side::Old);
            side(ctx, ui, right, &shown, Side::New);
        }
        DiffView::Inline => side(ctx, ui, body, &shown, Side::New),
    }
}

struct Shown<'a> {
    review: &'a Review,
    key: &'a str,
    view: DiffView,
    /// How far it is scrolled down and sideways.
    at: (f32, f32),
}

/// One column of rows: the whole change inline, or one side of it in split.
fn side(ctx: &mut Ctx, ui: &Ui, rect: Rect, shown: &Shown, pane: Side) {
    let file = &shown.review.aligned;
    let total = file.len();
    let extent = (height(ctx, total) - rect.h).max(0.0);
    ctx.app.hits.scrolls(Scroller::Sheet, extent);
    let scroll = shown.at.0.min(extent);
    let window = visible(ctx, rect, total, scroll);
    let colours = coloured(ui, shown, window.clone());
    let rows: Vec<Drawn> = window
        .clone()
        .map(|at| {
            let side = file
                .row(at)
                .map_or(pane, |one| source(&one, shown.view, pane));
            let drawn = plain(file, at, (shown.view, side), Some(&colours), |_, _| None);
            drawn.map(|(drawn, _)| drawn).unwrap_or_default()
        })
        .collect();
    let gutters = gutters_of(&rows);
    let lines: Vec<Line> = rows
        .iter()
        .zip(&gutters)
        .map(|(row, gutters)| lined(ctx, row, gutters))
        .collect();
    let numbers = numbers(shown);
    let texts = rows.iter().map(|row| row.text.as_str());
    let reach = across_extent(ctx, numbers, rect, texts);
    ctx.app.hits.scrolls(Scroller::SheetAcross, reach);
    let rows = Rows {
        lines: &lines,
        first: window.start,
        gutters: numbers,
        across: shown.at.1.min(reach),
    };
    code(ctx, rect, rows, scroll);
}

/// Both sides' colours for the lines `window` shows.
fn coloured(ui: &Ui, shown: &Shown, window: Range<usize>) -> (Rc<Colours>, Rc<Colours>) {
    let file = &shown.review.aligned;
    let (mut old, mut new) = (0..0, 0..0);
    for row in window.filter_map(|at| file.row(at)) {
        stretch(&mut old, row.old);
        stretch(&mut new, row.new);
    }
    let review = shown.review;
    let painted =
        |doc, old_side, lines| ui.painted.of(doc, shown.key, old_side, (0, (0, 0)), lines);
    (
        painted(&review.old, true, old),
        painted(&review.new, false, new),
    )
}

/// One number column a side in split, two in inline, wide enough for the longer side.
fn numbers(shown: &Shown) -> Gutters {
    let review = shown.review;
    let longest = review.old.lines().max(review.new.lines());
    Gutters {
        cells: match shown.view {
            DiffView::Inline => 2,
            DiffView::Split => 1,
        },
        digits: longest.to_string().len(),
        marks: false,
        folds: false,
    }
}
