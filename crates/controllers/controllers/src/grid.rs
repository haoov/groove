//! A selection of our own on a terminal's grid, the agent's or a shell's.

use groove_agent_service::{Select, Terminal};

use crate::{Services, Spawner};

/// A selection begun at a cell, or carried to one.
pub(crate) fn select(terminal: &Terminal, cell: (usize, usize), kind: Select, from: bool) {
    match from {
        true => terminal.select_from(cell, kind),
        false => terminal.select_to(cell),
    }
}

/// What is selected on the grid, onto the clipboard.
pub(crate) fn copy(terminal: &Terminal, services: &Services, spawner: &dyn Spawner) {
    if let Some(said) = terminal.selected() {
        crate::workspace::copied(services, spawner, said);
    }
}
