//! Where the rows are drawn: one column, or the two sides beside each other.

use groove_gfx::Rect;
use groove_types::{DiffView, RowKind};

use groove_controllers::AppState;

use super::row::{Side, count, drawn};
use crate::Ui;
use crate::ctx::Ctx;
use crate::hit::{Scroller, Target};
use crate::widget::{Gutters, Line, Rows, chars_of, code, height, visible};

pub(super) fn rows(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    match ui.session.view {
        DiffView::Split => beside(ctx, body, app, ui),
        view => surface(ctx, body, app, ui, view, Side::New, true),
    }
}

/// The old on the left, the new on the right, one alignment between them.
fn beside(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui) {
    let thickness = ctx.tokens.hairline;
    let half = ((body.w - thickness) / 2.0).floor();
    let left = Rect::new(body.x, body.y, half, body.h);
    let right = Rect::new(
        left.right() + thickness,
        body.y,
        body.w - half - thickness,
        body.h,
    );
    let rule = ctx.styles.line();
    ctx.quad(Rect::new(left.right(), body.y, thickness, body.h), rule);
    let view = DiffView::Split;
    surface(ctx, left, app, ui, view, Side::Old, false);
    surface(ctx, right, app, ui, view, Side::New, true);
}

/// Draws the rows the surface has room for, and says where a click can land.
fn surface(
    ctx: &mut Ctx,
    rect: Rect,
    app: &AppState,
    ui: &Ui,
    view: DiffView,
    side: Side,
    clickable: bool,
) {
    let total = count(app, view);
    let extent = (height(ctx, total) - rect.h).max(0.0);
    ctx.scrolls(Scroller::Code, extent);
    let scroll = ui.session.diff.min(extent);
    let window = visible(ctx, rect, total, scroll);
    let rows = drawn(app, ui, view, side, window.clone());
    let gutters: Vec<Vec<&str>> = rows
        .iter()
        .map(|row| row.gutters.iter().map(String::as_str).collect())
        .collect();
    let lines: Vec<Line<'_>> = rows
        .iter()
        .enumerate()
        .map(|(at, row)| {
            if row.head {
                return Line::head(&row.text);
            }
            if let RowKind::Gap(_) = row.kind {
                return Line::banner(&row.text);
            }
            let line = Line::new(&row.text).gutters(&gutters[at]).spans(&row.spans);
            let line = line
                .mark(row.mark.map(|mark| ctx.styles.mark(mark)))
                .caret(row.caret)
                .held(row.held);
            match ctx.styles.row_ground(row.kind) {
                Some(color) => line.ground(color),
                None => line,
            }
        })
        .collect();
    let numbers = numbers(app, view);
    if clickable {
        ctx.showing(window.clone());
        ctx.hit(rect, Target::Code);
        let chars = chars_of(ctx, numbers, rect, scroll);
        ctx.characters(chars);
    }
    let rows = Rows {
        lines: &lines,
        first: window.start,
        gutters: numbers,
    };
    code(ctx, rect, rows, scroll);
}

/// How wide the numbers stand: one column a side in split and file, two in inline.
/// The whole change shares one width, so the text does not shift file to file.
pub(super) fn numbers(app: &AppState, view: DiffView) -> Gutters {
    let digits = match view {
        DiffView::File => match app.workspace.opened.as_ref() {
            Some(file) => file.new.lines().to_string().len(),
            None => 1,
        },
        _ => app.workspace.changes.digits(),
    };
    Gutters {
        cells: match view {
            DiffView::Inline => 2,
            _ => 1,
        },
        digits,
    }
}
