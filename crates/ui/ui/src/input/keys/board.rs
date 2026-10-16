//! The keys of the board's columns: walk the rows, open one, let it go.

use groove_controllers::{AppState, Command};

use super::super::{Key, Modifiers};
use crate::Ui;
use crate::hit::{Hits, Scroller};
use crate::views::board::List;
use groove_ui_kit::base::ctx::Metrics;

/// The column picked, or the review table: the arrows walk it, Enter goes, ctrl+Enter stays.
pub(super) fn chosen(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    (hits, metrics): (&Hits, Metrics),
) -> Vec<Command> {
    let list = ui.board.list.unwrap_or(List::Review);
    let rows = crate::views::board::spans(app, ui, list, &metrics.tokens());
    let at = rows
        .iter()
        .position(|(one, ..)| ui.hover.as_ref() == Some(one));
    match key {
        Key::Up | Key::Down => {
            let next = match at {
                None => (!rows.is_empty()).then_some(0),
                at => super::panes::stepped(key, at, rows.len()),
            };
            if let Some(next) = next {
                ui.hover = Some(rows[next].0.clone());
                followed(ui, list, (&rows, next), hits);
            }
            Vec::new()
        }
        Key::Enter => match at.map(|at| rows[at].0.clone()) {
            Some(target) => crate::input::pointer::opened_from_board(target, ui, !mods.ctrl),
            None => Vec::new(),
        },
        Key::Escape => {
            ui.hover = None;
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// The column scrolled as little as it takes to show row `at` whole.
fn followed(
    ui: &mut Ui,
    list: List,
    (rows, at): (&[(crate::hit::Target, f32, f32)], usize),
    hits: &Hits,
) {
    let extent = hits.extent(Scroller::Column(list as u8));
    let total = rows.last().map_or(0.0, |(_, top, tall)| top + tall);
    let room = (total - extent).max(0.0);
    let (_, top, tall) = &rows[at];
    let offset = ui.board.scroll(list).min(*top).max(top + tall - room);
    ui.board.scrolled(list, offset.clamp(0.0, extent.max(0.0)));
}

/// The offset that shows row `at`, moved as little as it can be.
pub(super) fn kept(offset: f32, (at, row): (usize, f32), (count, extent): (usize, f32)) -> f32 {
    if extent <= 0.0 {
        return 0.0;
    }
    let room = count as f32 * row - extent;
    let top = at as f32 * row;
    let offset = offset.min(top);
    offset.max(top + row - room).clamp(0.0, extent)
}
