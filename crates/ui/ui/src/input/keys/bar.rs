//! The keys of the two search bars: the sidebar's terms and the bar over the rows.

use groove_controllers::{AppState, Command, workspace};
use groove_types::{Caret, Edit, Motion, Selection};

use super::super::{Key, Modifiers};
use crate::hit::{Hits, Scroller, Target};
use crate::input::follow::across_at;
use crate::keymap::{Action, Keymap};
use crate::views::session::diff::Inline;
use crate::views::session::find::Finding;
use crate::views::session::{Face, Term};
use crate::{Focus, Ui};
use groove_ui_kit::base::ctx::Metrics;
use groove_ui_kit::widgets::Field;

/// The find bar's keys: it takes what is typed until `Enter`, and the chords step either way.
pub(super) fn finding(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    keymap: &Keymap,
    seen: (&Hits, Metrics),
) -> Option<Vec<Command>> {
    if let Some(commands) = super::resources::chord((key, mods), ui, app, keymap) {
        return Some(commands);
    }
    let view = ui.session.face();
    if keymap.is(Action::Find, key, mods) && ui.focus == Focus::Workspace {
        let find = ui.session.find.get_or_insert_with(|| Finding::open(view));
        find.typing = true;
        return Some(Vec::new());
    }
    let find = ui.session.find.as_mut()?;
    if ui.focus != Focus::Workspace && !find.typing {
        return None;
    }
    match key {
        Key::Escape => {
            ui.session.find = None;
            return Some(Vec::new());
        }
        key if keymap.is(Action::FindNext, key, mods) => find.step(true),
        key if keymap.is(Action::FindPrevious, key, mods) => find.step(false),
        Key::Enter if find.typing => find.typing = false,
        key if find.typing => match typing(key, mods, &mut find.query) {
            true => searched(ui, app, view),
            false => return Some(Vec::new()),
        },
        _ => return None,
    }
    Some(reached(ui, app, seen))
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
fn searched(ui: &mut Ui, app: &AppState, view: crate::views::session::Face) {
    let query = ui
        .session
        .find
        .as_ref()
        .map(|find| find.query.text().to_string());
    let hits = crate::views::session::find::found(app, ui, view, &query.unwrap_or_default());
    if let Some(find) = ui.session.find.as_mut() {
        (find.hits, find.view, find.at) = (hits, view, 0);
    }
}

/// The surface scrolled to centre the match it stands on, which it holds as a selection.
fn reached(ui: &mut Ui, app: &AppState, (hits, metrics): (&Hits, Metrics)) -> Vec<Command> {
    let Some(find) = ui.session.find.as_ref() else {
        return Vec::new();
    };
    let Some(hit) = find.here().cloned() else {
        return Vec::new();
    };
    let holds =
        crate::views::session::diff::edited(app, ui).is_some_and(|one| one.path == hit.path);
    let row = Inline::of(app, ui, find.view, hits.wrap()).shifted(hit.row);
    let same = holds || find.view != Face::File;
    *ui.session.scroll_mut() = centred(row, metrics.tokens().line, hits, same);
    let Some(at) = hit.line.map(|line| Caret::new(line, hit.range.start)) else {
        return Vec::new();
    };
    let end = Caret::new(at.line, hit.range.end);
    *ui.session.across_mut() = across_at(ui, app, (&hit.path, end), (hits, metrics));
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
    let editing = crate::editor::Editing::keyed(app, ui).unwrap_or(crate::editor::Editing::File);
    [Edit::Move(Motion::To(at)), Edit::Extend(Motion::To(end))]
        .into_iter()
        .map(|edit| editing.edit(edit))
        .collect()
}

/// The scroll that puts `row` in the middle of the rows drawn, within them when they are the same.
fn centred(row: usize, line: f32, hits: &Hits, same: bool) -> f32 {
    let tall = hits.rect_of(&Target::Code).map_or(0.0, |rows| rows.h);
    let top = row as f32 * line - (tall - line) / 2.0;
    match same {
        true => top.clamp(0.0, hits.extent(Scroller::Code).max(0.0)),
        false => top.max(0.0),
    }
}

/// The bar open on one term, on the tab that shows it; the board has none.
pub(super) fn opened(ui: &mut Ui, app: &AppState, term: Term) {
    if ui.showing(app) != crate::Surface::Session {
        return;
    }
    if !ui.session.tab.has_sidebar() {
        ui.session.tab = crate::views::session::Tab::Diff;
    }
    ui.session.folded = false;
    ui.session.bar.open(term);
    ui.session.composing = false;
}

/// What a keystroke asks of the bar: a term narrowed, the search run again, or a file opened.
pub(super) fn in_bar(
    key: Key,
    mods: Modifiers,
    ui: &mut Ui,
    app: &AppState,
    keymap: &Keymap,
) -> Vec<Command> {
    let Some(term) = ui.session.bar.typing else {
        return Vec::new();
    };
    match key {
        Key::Escape => ui.session.bar.shut(),
        Key::Tab => ui.session.bar.open(other(term)),
        key if keymap.is(Action::OpenPath, key, mods) => ui.session.bar.open(Term::Path),
        key if keymap.is(Action::SearchFiles, key, mods) => ui.session.bar.open(Term::Text),
        Key::Enter if ui.session.bar.greps() => ui.session.bar.typing = None,
        Key::Enter => {
            let first = crate::views::session::files::narrowed(app, ui)
                .first()
                .map(|file| file.path.clone());
            ui.session.bar.typing = None;
            let Some(path) = first else {
                return Vec::new();
            };
            if app.workspace.changes.head_of(&path).is_none() {
                ui.session.tab = crate::views::session::Tab::Files;
            }
            return vec![Command::Workspace(workspace::Command::OpenFile {
                path,
                at: None,
            })];
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

/// What the bar asks for: the worktree's files, and the search while it holds text.
fn searches(ui: &Ui) -> Vec<Command> {
    let bar = &ui.session.bar;
    let mut asks = Vec::new();
    if bar.path.text().chars().count() == 1 {
        asks.push(Command::Workspace(workspace::Command::ListPaths));
    }
    if bar.greps() {
        asks.push(Command::Workspace(workspace::Command::Grep {
            query: bar.text.text().to_string(),
            under: bar.path.text().to_string(),
        }));
    }
    asks
}
