//! Keeping the reader on the same line when the view changes.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Opened;

use crate::views::session::Face;

/// The scroll that keeps the same line in view once the view changes.
pub(crate) fn scrolled(
    app: &AppState,
    ui: &crate::Ui,
    (from, to): (Face, Face),
    (scroll, line): (f32, f32),
    cols: usize,
) -> f32 {
    let at = (scroll / line).floor().max(0.0) as usize;
    moved(app, ui, (from, to), at, cols) as f32 * line
}

/// The row that holds the same line once the view changes.
pub(crate) fn moved(
    app: &AppState,
    ui: &crate::Ui,
    (from, to): (Face, Face),
    row: usize,
    cols: usize,
) -> usize {
    let there = super::notes::Inline::of(app, ui, to, cols);
    let row = super::notes::Inline::of(app, ui, from, cols).base(row);
    let Some(file) = app.workspace.active() else {
        return there.shifted(row);
    };
    let Some(number) = number(file, from, row) else {
        return there.shifted(row);
    };
    let at = match to {
        Face::File => number as usize,
        _ => file
            .hunked
            .layout
            .all()
            .position(|at| at.new.is_some_and(|line| line >= number))
            .unwrap_or(row),
    };
    there.shifted(at)
}

/// The new-side line the top of the view is on.
fn number(file: &Opened, view: Face, at: usize) -> Option<u32> {
    match view {
        Face::File => Some(at as u32),
        _ => file
            .hunked
            .layout
            .rows(at..file.hunked.layout.len())
            .find_map(|row| row.new),
    }
}
