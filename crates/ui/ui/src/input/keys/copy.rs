//! The terminal selection copied with the keys.

use groove_controllers::{AppState, Command};

use crate::{Focus, Grid};

/// The selection of the terminal that has the keys, to the clipboard.
pub(super) fn copied(focus: Focus, app: &AppState) -> Vec<Command> {
    let grid = match focus {
        Focus::Terminal => app
            .session
            .selected
            .as_ref()
            .and_then(|session| app.shell.shells(session)?.focused())
            .map(Grid::Shell),
        _ => Some(Grid::Agent),
    };
    grid.map_or_else(Vec::new, |grid| crate::input::selection::copied(app, grid))
}
