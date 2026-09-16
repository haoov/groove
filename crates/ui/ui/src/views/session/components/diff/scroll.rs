//! Keeping the reader on the same line when the view changes.

use groove_controllers::AppState;
use groove_controllers::workspace_service::Opened;
use groove_types::DiffView;

/// The scroll that keeps the same line in view once the view changes.
pub(crate) fn scrolled(
    app: &AppState,
    from: DiffView,
    to: DiffView,
    scroll: f32,
    line: f32,
) -> f32 {
    let at = (scroll / line).floor().max(0.0) as usize;
    moved(app, from, to, at) as f32 * line
}

/// The row that holds the same line once the view changes.
pub(crate) fn moved(app: &AppState, from: DiffView, to: DiffView, row: usize) -> usize {
    let Some(file) = app.workspace.opened.as_ref() else {
        return row;
    };
    let Some(number) = number(file, from, row) else {
        return row;
    };
    match to {
        DiffView::File => number as usize,
        _ => file
            .rows
            .iter()
            .position(|at| at.new.is_some_and(|line| line >= number))
            .unwrap_or(row),
    }
}

/// The new-side line the top of the view is on.
fn number(file: &Opened, view: DiffView, at: usize) -> Option<u32> {
    match view {
        DiffView::File => Some(at as u32),
        _ => file.rows.get(at..)?.iter().find_map(|row| row.new),
    }
}
