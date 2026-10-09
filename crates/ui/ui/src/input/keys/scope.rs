//! A scope panel has the keys: the search narrows, the arrows step, space shows, Enter shows alone.

use groove_controllers::{AppState, Command};

use super::super::{Key, Modifiers};
use super::bar::typing;
use crate::views::session::resources::{How, ScopeLine, choose, scope_lines};
use crate::{Overlay, Ui};

pub(super) fn in_scope(
    key: Key,
    mods: Modifiers,
    ui: &mut Ui,
    app: &AppState,
) -> Option<Vec<Command>> {
    let open = app.session.selected()?;
    let Some(Overlay::Scope(scoping)) = &mut ui.overlay else {
        return None;
    };
    let lines = scope_lines(app, open, &ui.session.resources, scoping);
    let picks: Vec<usize> = (0..lines.len())
        .filter(|at| !matches!(lines[*at], ScopeLine::Heading(_)))
        .collect();
    let how = match key {
        Key::Escape => {
            ui.overlay = None;
            return Some(Vec::new());
        }
        Key::Down | Key::Up => {
            scoping.cursor = stepped(&picks, scoping.cursor, key == Key::Down);
            return Some(Vec::new());
        }
        Key::Enter => How::Alone,
        Key::Char(' ') => How::Toggle,
        key => {
            typing(key, mods, &mut scoping.query);
            let lines = scope_lines(app, open, &ui.session.resources, scoping);
            let first = lines
                .iter()
                .position(|one| !matches!(one, ScopeLine::Heading(_)));
            scoping.cursor = first.filter(|_| !scoping.query.is_empty());
            return Some(Vec::new());
        }
    };
    let typed = picks.first().copied().filter(|_| !scoping.query.is_empty());
    let Some(at) = scoping.cursor.or(typed) else {
        return Some(Vec::new());
    };
    Some(pick((at, how), ui, app))
}

/// The next line that acts, down or up from the cursor; the first one when there is none.
fn stepped(picks: &[usize], cursor: Option<usize>, down: bool) -> Option<usize> {
    let Some(cursor) = cursor else {
        return picks.first().copied();
    };
    let next = match down {
        true => picks.iter().find(|at| **at > cursor),
        false => picks.iter().rev().find(|at| **at < cursor),
    };
    next.copied().or(Some(cursor))
}

/// Line `at` chosen `how`; a held one shown alone shuts the panel onto the Resources tab.
pub(in crate::input) fn pick((at, how): (usize, How), ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let Some(open) = app.session.selected() else {
        return Vec::new();
    };
    let Some(Overlay::Scope(scoping)) = &mut ui.overlay else {
        return Vec::new();
    };
    let lines = scope_lines(app, open, &ui.session.resources, scoping);
    let Some(line) = lines.get(at) else {
        return Vec::new();
    };
    let commands = choose(line, how, (open, &mut ui.session.resources, scoping));
    if how == How::Alone && matches!(line, ScopeLine::Held(_)) {
        ui.overlay = None;
        ui.session.tab = crate::views::session::Tab::Resources;
    }
    commands
}
