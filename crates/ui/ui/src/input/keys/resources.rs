//! The Resources tab's chords while the workspace has the keys, and typing in its searches.

use groove_controllers::{AppState, Command};

use super::super::{Key, Modifiers};
use crate::hit::{Hits, Scroller};
use crate::keymap::{Action, Keymap};
use crate::views::session::Tab;
use crate::views::session::resources::{Line, Scoping};
use crate::{Focus, Overlay, Surface, Ui};
use groove_ui_kit::base::ctx::Metrics;

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
pub(super) fn in_resources(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    seen: (&Hits, Metrics),
) -> Vec<Command> {
    if ui.session.resources.filtering {
        return in_kinds((key, mods), ui, app, seen);
    }
    let held = &mut ui.session.resources;
    match key {
        Key::Enter => {
            held.typing = false;
            let first = crate::views::session::resources::row_uids(app, ui)
                .into_iter()
                .next();
            ui.hover = first.map(crate::hit::Target::ResourceRow);
        }
        Key::Escape => {
            held.finding &= !held.search.is_empty();
            held.typing = false;
        }
        key => {
            super::bar::typing(key, mods, &mut held.search);
            search_changed(ui);
        }
    }
    Vec::new()
}

/// The kind search: arrows step through the lines; Enter folds a heading or lists a kind.
fn in_kinds(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    (hits, metrics): (&Hits, Metrics),
) -> Vec<Command> {
    let shown = crate::views::session::resources::shown(app, ui);
    let held = &mut ui.session.resources;
    let last = shown.len().checked_sub(1);
    let in_sight = |held: &mut crate::views::session::resources::ResourcesUi| {
        if let Some(at) = held.cursor {
            let extent = hits.extent(Scroller::Kinds);
            let row = (at, metrics.tokens().row);
            held.kinds = super::board::kept(held.kinds, row, (shown.len(), extent));
        }
    };
    match key {
        Key::Down => {
            held.cursor = held
                .cursor
                .map_or(Some(0), |at| Some(at + 1))
                .zip(last)
                .map(|(at, end)| at.min(end));
            in_sight(held);
        }
        Key::Up => {
            held.cursor = held.cursor.map(|at| at.saturating_sub(1));
            in_sight(held);
        }
        Key::Enter => match held.cursor.and_then(|at| shown.get(at)) {
            Some(Line::Heading(heading, _)) => {
                groove_types::toggle(&mut held.folded, heading.clone());
            }
            Some(Line::Kind(kind)) => {
                (held.kind, held.scroll) = (Some(kind.clone()), 0.0);
                (held.filtering, held.cursor) = (false, None);
                ui.focus = Focus::Workspace;
            }
            None => (held.filtering, held.cursor) = (false, None),
        },
        Key::Escape => (held.filtering, held.cursor) = (false, None),
        key => {
            super::bar::typing(key, mods, &mut held.filter);
            kinds_changed(ui, app);
        }
    }
    Vec::new()
}

/// The list search's text changed: the rows back at the top.
pub(in crate::input) fn search_changed(ui: &mut Ui) {
    ui.session.resources.scroll = 0.0;
}

/// The kind search's text changed: the cursor on the first kind it keeps.
pub(in crate::input) fn kinds_changed(ui: &mut Ui, app: &AppState) {
    let lines = crate::views::session::resources::shown(app, ui);
    let first = lines.iter().position(|one| matches!(one, Line::Kind(_)));
    ui.session.resources.cursor = first;
}

/// The kinds sidebar has the keys: the arrows walk it, Enter lists or folds, a letter searches.
pub(super) fn in_kinds_list(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    seen: (&Hits, Metrics),
) -> Vec<Command> {
    match key {
        Key::Char(_) if !mods.ctrl && !mods.alt => {
            ui.session.resources.filtering = true;
            in_kinds((key, mods), ui, app, seen)
        }
        Key::Up | Key::Down | Key::Enter => in_kinds((key, mods), ui, app, seen),
        _ => Vec::new(),
    }
}

/// The list has the keys: the arrows walk its rows, Enter describes one, `y` and `l` open its
/// YAML and its logs; `None` for any other key.
pub(super) fn in_table(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    (hits, metrics): (&Hits, Metrics),
) -> Option<Vec<Command>> {
    let held = &ui.session.resources;
    let listed = ui.session.tab == Tab::Resources && held.showing.is_none() && !held.typing;
    if !listed || mods.ctrl || mods.alt {
        return None;
    }
    let rows = crate::views::session::resources::row_uids(app, ui);
    let row = |uid: &String| crate::hit::Target::ResourceRow(uid.clone());
    let at = rows
        .iter()
        .position(|uid| ui.hover.as_ref() == Some(&row(uid)));
    let view = match key {
        Key::Up | Key::Down => {
            let next = match at {
                None => (!rows.is_empty()).then_some(0),
                at => super::panes::stepped(key, at, rows.len()),
            }?;
            ui.hover = Some(row(&rows[next]));
            followed(ui, next, hits, metrics);
            return Some(Vec::new());
        }
        Key::Enter => crate::views::session::resources::View::Describe,
        Key::Char('y') => crate::views::session::resources::View::Yaml,
        Key::Char('l') if pods(ui) => crate::views::session::resources::View::Logs,
        _ => return None,
    };
    let uid = rows.get(at?)?;
    let open = app.session.selected()?;
    let link = crate::views::session::resources::row_link(app, open, &ui.session.resources, uid)?;
    ui.session.resources.open(link);
    if let Some(tab) = ui.session.resources.tab_mut() {
        tab.view = view;
    }
    Some(Vec::new())
}

/// Whether the list shows pods, whose rows have logs.
fn pods(ui: &Ui) -> bool {
    let kind = ui.session.resources.kind.as_ref();
    kind.is_none_or(|one| one.kind == "Pod")
}

/// The list scrolled until the row at `at` stands whole inside it.
fn followed(ui: &mut Ui, at: usize, hits: &Hits, metrics: Metrics) {
    let row = metrics.tokens().row;
    let target = ui.hover.clone();
    let bottom = crate::layout::Layout::of(metrics, ui).workspace.bottom();
    let held = &mut ui.session.resources;
    let top = at as f32 * row;
    held.scroll = match target.and_then(|one| hits.rect_of(&one)) {
        Some(rect) if rect.bottom() > bottom => held.scroll + rect.bottom() - bottom,
        Some(_) => held.scroll,
        None if top < held.scroll => top,
        None => held.scroll + row,
    };
}
