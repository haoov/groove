//! The chords the Resources tab hears while the workspace has the keys.

use groove_controllers::{AppState, Command};

use super::super::{Key, Modifiers};
use crate::keymap::{Action, Keymap};
use crate::views::session::Tab;
use crate::views::session::resources::{Line, Scoping};
use crate::{Focus, Overlay, Surface, Ui};

/// The list's find bar, the kind search, or the namespace picker; `None` for any other chord.
pub(super) fn chord(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    keymap: &Keymap,
) -> Option<Vec<Command>> {
    let shown = ui.focus == Focus::Workspace && ui.showing(app) == Surface::Session;
    let (open, true) = (app.session.selected()?, shown) else {
        return None;
    };
    let up = ui.session.tab == Tab::Resources && open.holds_clusters();
    let held = &mut ui.session.resources;
    match () {
        _ if up && held.showing.is_none() && keymap.is(Action::Find, key, mods) => {
            (held.finding, held.typing, held.filtering) = (true, true, false);
        }
        _ if up && keymap.is(Action::OpenPath, key, mods) => {
            (held.filtering, held.typing) = (true, false);
        }
        _ if keymap.is(Action::SelectNamespace, key, mods) => {
            let (scoping, commands) = Scoping::asked(open).opened(open, held);
            ui.overlay = Some(Overlay::Scope(scoping));
            return Some(commands);
        }
        _ => return None,
    }
    Some(Vec::new())
}

/// A Resources search has the keys: Enter gives them back; Esc too, and shuts an empty find bar.
pub(super) fn in_resources(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if ui.session.resources.filtering {
        return in_kinds(key, mods, ui, app);
    }
    let held = &mut ui.session.resources;
    match key {
        Key::Enter => held.typing = false,
        Key::Escape => {
            held.finding &= !held.search.is_empty();
            held.typing = false;
        }
        key => {
            super::bar::typing(key, mods, &mut held.search);
            held.scroll = 0.0;
        }
    }
    Vec::new()
}

/// The kind search: arrows step through the lines; Enter folds a heading or lists a kind.
fn in_kinds(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let shown = crate::views::session::resources::shown(app, ui);
    let held = &mut ui.session.resources;
    let last = shown.len().checked_sub(1);
    match key {
        Key::Down => {
            held.cursor = held
                .cursor
                .map_or(Some(0), |at| Some(at + 1))
                .zip(last)
                .map(|(at, end)| at.min(end))
        }
        Key::Up => held.cursor = held.cursor.map(|at| at.saturating_sub(1)),
        Key::Enter => match held.cursor.and_then(|at| shown.get(at)) {
            Some(Line::Heading(heading, _)) => {
                groove_types::toggle(&mut held.folded, heading.clone());
            }
            Some(Line::Kind(kind)) => {
                (held.kind, held.scroll) = (Some(kind.clone()), 0.0);
                (held.filtering, held.cursor) = (false, None);
            }
            None => (held.filtering, held.cursor) = (false, None),
        },
        Key::Escape => (held.filtering, held.cursor) = (false, None),
        key => {
            super::bar::typing(key, mods, &mut held.filter);
            let lines = crate::views::session::resources::shown(app, ui);
            let first = lines.iter().position(|one| matches!(one, Line::Kind(_)));
            ui.session.resources.cursor = first;
            return Vec::new();
        }
    }
    Vec::new()
}
