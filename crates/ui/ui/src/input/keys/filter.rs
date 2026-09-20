//! The keys of the board's filter: what it takes, and what it offers.

use groove_controllers::{AppState, Command};

use super::super::{Key, Modifiers};
use super::typing;
use crate::Ui;
use crate::views::board::complete;

/// What a key does while the board is up: the filter takes it, or `/` opens it.
pub(super) fn on_board(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if !ui.board.typing {
        if matches!(key, Key::Char('/')) {
            ui.board.focus();
        }
        return Vec::new();
    }
    let offers = complete::offers(app, ui.board.filter.text());
    let shown = offers.len().min(complete::ROWS);
    match key {
        Key::Escape => shut(ui),
        Key::Down => ui.board.offer = step(ui.board.offer, 1, shown),
        Key::Up => ui.board.offer = step(ui.board.offer, -1, shown),
        Key::Enter | Key::Tab => match offers.get(ui.board.offer).cloned() {
            Some(pick) => ui.board.take(&pick),
            None => ui.board.typing = false,
        },
        key => {
            if typing(key, mods, &mut ui.board.filter) {
                ui.board.offer = 0;
            }
        }
    }
    Vec::new()
}

/// Escape leaves the filter, then clears it.
fn shut(ui: &mut Ui) {
    match ui.board.filter.is_empty() {
        true => ui.board.typing = false,
        false => ui.board.filter.clear(),
    }
}

fn step(from: usize, by: i32, shown: usize) -> usize {
    if shown == 0 {
        return 0;
    }
    let last = shown - 1;
    match by {
        1 => (from + 1) % shown,
        _ => from.checked_sub(1).unwrap_or(last),
    }
}
