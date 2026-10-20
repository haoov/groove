//! The heads of a split diff, drawn once across both sides.

use groove_controllers::AppState;
use groove_controllers::workspace_service::{At, HEAD_ROWS};
use groove_gfx::Rect;

use super::notes::{Inline, Slot};
use super::row::{count, counted, is_read};
use crate::Ui;
use crate::components::{Gutters, Line, Rows, code, head_of, height, visible};
use crate::ctx::Ctx;

/// Every head in view across `body`, over the two sides that leave their rows to it.
pub(super) fn across(ctx: &mut Ctx, body: Rect, app: &AppState, ui: &Ui, inline: &Inline) {
    let total = inline.total(count(app, ui, ui.session.face()));
    let scroll = ui
        .session
        .scroll()
        .min((height(ctx, total) - body.h).max(0.0));
    let window = visible(ctx, body, total, scroll);
    let mut named = Vec::new();
    let lines: Vec<Line<'_>> = window
        .clone()
        .enumerate()
        .map(|(on, row)| head(app, inline.slot(row), on, &mut named))
        .collect();
    let gutters = Gutters {
        cells: 0,
        digits: 1,
        marks: false,
        folds: false,
    };
    let rows = Rows {
        lines: &lines,
        first: window.start,
        gutters,
        across: 0.0,
    };
    let drawn = code(ctx, body, rows, scroll);
    ctx.clipped(body, |ctx| {
        for (on, part, path) in named {
            super::offers::head_hits(ctx, head_of(drawn[on], (part, HEAD_ROWS)), path);
        }
    });
}

/// The head a row of the window draws, named once at its first row on screen; else nothing.
fn head<'a>(
    app: &'a AppState,
    slot: Slot,
    on: usize,
    named: &mut Vec<(usize, usize, &'a str)>,
) -> Line<'a> {
    let changes = &app.workspace.changes;
    let Slot::Code(at) = slot else {
        return Line::covered();
    };
    let Some(At::Head(file)) = changes.at(at) else {
        return Line::covered();
    };
    let part = changes.head_of(&file.path).map_or(0, |head| at - head);
    if part > 0 && on > 0 {
        return Line::covered();
    }
    named.push((on, part, file.path.as_str()));
    Line::head(&file.path)
        .folded(changes.is_folded(&file.path))
        .read(is_read(app, &file.path))
        .part((part, HEAD_ROWS))
        .counts(counted(app, &file.path))
}
