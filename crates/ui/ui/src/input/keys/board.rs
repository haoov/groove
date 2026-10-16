//! The keys of the board's review table: move the selection, open it, let it go.

use groove_controllers::{AppState, Command, session};

use super::super::{Key, Modifiers};
use crate::Ui;
use crate::hit::{Hits, Scroller};
use crate::views::board::List;
use crate::views::board::review::{chosen as at, sorted};
use groove_ui_kit::base::ctx::Metrics;

pub(super) fn chosen(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    (hits, metrics): (&Hits, Metrics),
) -> Vec<Command> {
    if let Some(list @ (List::Live | List::Next)) = ui.board.list {
        return walked((key, mods), ui, app, list);
    }
    let asked = sorted(app, ui);
    let Some(last) = asked.len().checked_sub(1) else {
        return Vec::new();
    };
    let now = at(ui, &asked);
    let next = match key {
        Key::Down => now.map_or(0, |now| (now + 1).min(last)),
        Key::Up => now.map_or(0, |now| now.saturating_sub(1)),
        Key::Escape => {
            ui.board.chosen = None;
            return Vec::new();
        }
        Key::Enter => {
            let Some(mr) = now.map(|now| asked[now]) else {
                return Vec::new();
            };
            let (project, iid) = (mr.project.clone(), mr.iid);
            if !mods.ctrl {
                ui.surface = crate::Surface::Session;
            }
            return vec![Command::Session(session::Command::OpenReview {
                project,
                iid,
            })];
        }
        _ => return Vec::new(),
    };
    ui.board.chosen = Some((asked[next].project.clone(), asked[next].iid));
    let row = crate::views::board::card(&metrics.tokens());
    let extent = hits.extent(Scroller::Column(List::Review as u8));
    ui.board.review = kept(ui.board.review, (next, row), (asked.len(), extent));
    Vec::new()
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

/// Live or Up next: the arrows walk the rows, Enter opens one and goes to it, ctrl+Enter stays.
fn walked((key, mods): (Key, Modifiers), ui: &mut Ui, app: &AppState, list: List) -> Vec<Command> {
    let rows = crate::views::board::targets(app, ui, list);
    let at = rows.iter().position(|one| ui.hover.as_ref() == Some(one));
    match key {
        Key::Up | Key::Down => {
            let next = match at {
                None => (!rows.is_empty()).then_some(0),
                at => super::panes::stepped(key, at, rows.len()),
            };
            ui.hover = next.and_then(|next| rows.get(next)).cloned();
            Vec::new()
        }
        Key::Enter => match at.and_then(|at| rows.get(at)).cloned() {
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
