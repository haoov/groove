//! The keys of the two search bars: the sidebar's terms and the bar over the rows.

use groove_controllers::{AppState, Command, workspace};
use groove_types::{Caret, DiffView, Edit, Motion, Selection};

use super::super::{Key, Modifiers};
use crate::tokens::ABOVE_MATCH;
use crate::views::session::Term;
use crate::views::session::find::Finding;
use crate::widget::Field;
use crate::{Focus, Ui};

/// The find bar's own keys, while the workspace holds the keyboard: the bar takes
/// what is typed until `Enter` hands the code back, and the chords step either way.
pub(super) fn finding(
    key: Key,
    mods: Modifiers,
    ui: &mut Ui,
    app: &AppState,
) -> Option<Vec<Command>> {
    let view = ui.session.view;
    if mods.ctrl && matches!(key, Key::Char('f' | 'F')) && ui.focus == Focus::Workspace {
        let find = ui.session.find.get_or_insert_with(|| Finding::open(view));
        find.typing = true;
        return Some(Vec::new());
    }
    let find = ui.session.find.as_mut()?;
    match key {
        Key::Escape => {
            ui.session.find = None;
            return Some(Vec::new());
        }
        Key::Char('n' | 'N') if mods.ctrl => find.step(true),
        Key::Char('p' | 'P') if mods.ctrl => find.step(false),
        Key::Enter if find.typing => find.typing = false,
        key if find.typing => match typing(key, mods, &mut find.query) {
            true => searched(find, app, view),
            false => return Some(Vec::new()),
        },
        _ => return None,
    }
    Some(reached(ui, app))
}

/// One keystroke in a field. True when what it holds changed, which a motion does not.
pub(super) fn typing(key: Key, mods: Modifiers, field: &mut Field) -> bool {
    match key {
        Key::Left => field.left(),
        Key::Right => field.right(),
        Key::Home => field.home(),
        Key::End => field.end(),
        Key::Backspace => return took(field, Field::backspace),
        Key::Delete => return took(field, Field::delete),
        Key::Char(c) if !mods.ctrl && !mods.alt => return took(field, |field| field.insert(c)),
        _ => {}
    }
    false
}

fn took(field: &mut Field, act: impl FnOnce(&mut Field)) -> bool {
    act(field);
    true
}

/// The matches read again for what the bar now holds.
fn searched(find: &mut Finding, app: &AppState, view: DiffView) {
    find.hits = crate::views::session::find::found(app, view, find.query.text());
    find.view = view;
    find.at = 0;
}

/// The surface scrolled to the match it stands on, which it holds as a selection.
fn reached(ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let line = crate::tokens::Tokens::new(1.0).line;
    let Some(find) = ui.session.find.as_ref() else {
        return Vec::new();
    };
    let Some(hit) = find.here().cloned() else {
        return Vec::new();
    };
    let above = hit.row.saturating_sub(ABOVE_MATCH);
    ui.session.diff = above as f32 * line;
    let Some(at) = hit.line.map(|line| Caret::new(line, hit.range.start)) else {
        return Vec::new();
    };
    let end = Caret::new(at.line, hit.range.end);
    let holds = app
        .workspace
        .opened
        .as_ref()
        .is_some_and(|open| open.path == hit.path);
    if !holds {
        let open = workspace::Command::OpenFile {
            path: hit.path,
            at: Some(Selection {
                anchor: at,
                head: end,
            }),
        };
        return vec![Command::Workspace(open)];
    }
    [Edit::Move(Motion::To(at)), Edit::Extend(Motion::To(end))]
        .into_iter()
        .map(|edit| Command::Workspace(workspace::Command::Edit(edit)))
        .collect()
}

/// The bar open on one of its terms, the other kept as the scope it already is. Only
/// one thing takes what is typed, so the commit box gives the keyboard up.
pub(super) fn opened(ui: &mut Ui, term: Term) {
    ui.session.bar.open(term);
    ui.session.composing = false;
}

/// What a keystroke asks of the bar: a term narrowed, the search run again, or the
/// first file it left opened.
pub(super) fn in_bar(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let Some(term) = ui.session.bar.typing else {
        return Vec::new();
    };
    match key {
        Key::Escape => ui.session.bar.shut(),
        Key::Tab => ui.session.bar.open(other(term)),
        Key::Char('p' | 'P') if mods.ctrl => ui.session.bar.open(Term::Path),
        Key::Enter if ui.session.bar.greps() => ui.session.bar.typing = None,
        Key::Enter => {
            let first = crate::views::session::files::narrowed(app, ui)
                .first()
                .map(|file| file.path.clone());
            ui.session.bar.typing = None;
            return match first {
                Some(path) => vec![Command::Workspace(workspace::Command::OpenFile {
                    path,
                    at: None,
                })],
                None => Vec::new(),
            };
        }
        key => {
            if typing(key, mods, ui.session.bar.of(term)) {
                return searches(ui);
            }
        }
    }
    Vec::new()
}

fn other(term: Term) -> Term {
    match term {
        Term::Path => Term::Text,
        Term::Text => Term::Path,
    }
}

/// The worktree searched again for what the bar now holds, when it holds any text.
fn searches(ui: &Ui) -> Vec<Command> {
    let bar = &ui.session.bar;
    if !bar.greps() {
        return Vec::new();
    }
    let grep = workspace::Command::Grep {
        query: bar.text.text().to_string(),
        under: bar.path.text().to_string(),
    };
    vec![Command::Workspace(grep)]
}
