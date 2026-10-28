//! A scope panel has the keys: the search narrows, the arrows step, space shows, Enter shows alone.

use groove_controllers::{AppState, Command};

use super::super::{Key, Modifiers};
use super::bar::typing;
use crate::views::session::resources::{How, ScopeLine, choose, scope_lines};
use crate::{Focus, Overlay, Ui};

pub(super) fn in_scope(
    key: Key,
    mods: Modifiers,
    ui: &mut Ui,
    app: &AppState,
) -> Option<Vec<Command>> {
    let Some(Overlay::Scope(scoping)) = &mut ui.overlay else {
        return None;
    };
    let Some(open) = app.session.selected().filter(|_| key != Key::Escape) else {
        ui.overlay = None;
        return Some(Vec::new());
    };
    let lines = scope_lines(app, open, scoping);
    let picks = picks(&lines);
    let at = scoping.cursor.or(picks.first().copied());
    let how = match key {
        Key::Down | Key::Up => {
            scoping.cursor = stepped(&picks, at, key == Key::Down);
            return Some(Vec::new());
        }
        Key::Enter => How::Alone,
        Key::Char(' ') => How::Toggle,
        key => {
            if typing(key, mods, &mut scoping.query) {
                scoping.cursor = None;
            }
            return Some(Vec::new());
        }
    };
    let Some(at) = at else {
        return Some(Vec::new());
    };
    Some(pick((at, how), ui, app))
}

/// The lines that act, by their place among all the lines.
pub(in crate::input) fn picks(lines: &[ScopeLine]) -> Vec<usize> {
    let acts = |at: &usize| !matches!(lines[*at], ScopeLine::Heading(_));
    (0..lines.len()).filter(acts).collect()
}

/// The panel's query changed: the cursor back on the first line that acts.
pub(in crate::input) fn scope_changed(ui: &mut Ui) {
    if let Some(Overlay::Scope(scoping)) = &mut ui.overlay {
        scoping.cursor = None;
    }
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

/// Line `at` chosen `how`, the Kubernetes tab shown under it; Enter shuts the panel onto the list.
pub(in crate::input) fn pick((at, how): (usize, How), ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let Some(open) = app.session.selected() else {
        return Vec::new();
    };
    let Some(Overlay::Scope(scoping)) = &mut ui.overlay else {
        return Vec::new();
    };
    let lines = scope_lines(app, open, scoping);
    let Some(line) = lines.get(at) else {
        return Vec::new();
    };
    let commands = choose(line, how, (open, &mut ui.session.resources, scoping));
    if how == How::Detach || matches!(line, ScopeLine::Heading(_)) {
        return commands;
    }
    ui.session.tab = crate::views::session::Tab::Resources;
    ui.session.resources.showing = None;
    if how == How::Alone {
        ui.overlay = None;
        ui.focus = Focus::Workspace;
    }
    commands
}
