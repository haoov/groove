//! The sidebar's list from the keyboard: the arrows walk its rows, Enter does what a click
//! does, Space stages, Left and Right fold, `[` and `]` change the list.

use groove_controllers::{AppState, Command, workspace};

use super::super::Key;
use crate::hit::{Hits, Target};
use crate::layout::Layout;
use crate::views::session::files::{Stop, stops};
use crate::views::session::{Pane, Tab};
use crate::{Focus, Ui};
use groove_ui_kit::base::ctx::Metrics;

/// What the search bar passes to its list: the arrows, and Enter once a row is stood on.
pub(super) fn from_bar(
    key: Key,
    ui: &mut Ui,
    app: &AppState,
    seen: (&Hits, Metrics),
) -> Option<Vec<Command>> {
    let rows = stops(app, ui, &seen.1.tokens());
    let stood = rows
        .iter()
        .any(|one| ui.hover.as_ref() == Some(&one.target));
    match key {
        Key::Up | Key::Down => Some(in_list(key, ui, app, seen)),
        Key::Enter if stood => {
            ui.session.bar.typing = None;
            Some(in_list(key, ui, app, seen))
        }
        _ => None,
    }
}

pub(super) fn in_list(
    key: Key,
    ui: &mut Ui,
    app: &AppState,
    seen: (&Hits, Metrics),
) -> Vec<Command> {
    let rows = stops(app, ui, &seen.1.tokens());
    let at = rows
        .iter()
        .position(|one| ui.hover.as_ref() == Some(&one.target));
    match key {
        Key::Up | Key::Down => stepped(key, ui, app, (&rows, at), seen),
        Key::Char('[') | Key::Char(']') => paned(ui, key == Key::Char(']')),
        Key::Enter => match at {
            Some(at) => entered(rows[at].target.clone(), ui, app, seen),
            None => {
                ui.focus = Focus::Workspace;
                Vec::new()
            }
        },
        Key::Char(' ') => at.map_or_else(Vec::new, |at| staged(&rows[at].target, app)),
        Key::Left | Key::Right => {
            if let Some(at) = at {
                folded(&rows[at].target, ui, key == Key::Right);
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// The row above or below the one stood on, scrolled into view.
fn stepped(
    key: Key,
    ui: &mut Ui,
    app: &AppState,
    (rows, at): (&[Stop], Option<usize>),
    (hits, metrics): (&Hits, Metrics),
) -> Vec<Command> {
    let from = at.or_else(|| first_shown(rows, app, hits));
    let next = match (from, at) {
        (Some(from), None) => Some(from),
        (from, _) => super::panes::stepped(key, from, rows.len()),
    };
    let Some(stop) = next.and_then(|next| rows.get(next)) else {
        return Vec::new();
    };
    ui.hover = Some(stop.target.clone());
    followed(stop, ui, hits, metrics);
    Vec::new()
}

/// Where the arrows start when no row is stood on: the open file, else the first row drawn.
fn first_shown(rows: &[Stop], app: &AppState, hits: &Hits) -> Option<usize> {
    let open = app
        .workspace
        .active()
        .map(|one| Target::File(one.path.clone()));
    let open = rows
        .iter()
        .position(|one| Some(&one.target) == open.as_ref());
    let drawn = || {
        rows.iter()
            .position(|one| hits.rect_of(&one.target).is_some())
    };
    open.or_else(drawn).or((!rows.is_empty()).then_some(0))
}

/// The list scrolled until the row stands whole inside it.
fn followed(stop: &Stop, ui: &mut Ui, hits: &Hits, metrics: Metrics) {
    let bottom = Layout::of(metrics, ui).sidebar.bottom();
    let offset = ui.session.files;
    ui.session.files = match hits.rect_of(&stop.target) {
        Some(rect) if rect.bottom() > bottom => offset + rect.bottom() - bottom,
        Some(_) => offset,
        None if stop.top < offset => stop.top,
        None => offset + stop.height,
    };
}

/// Enter: what a click on the row does; a file hands the keys to the code.
fn entered(target: Target, ui: &mut Ui, app: &AppState, seen: (&Hits, Metrics)) -> Vec<Command> {
    let file = matches!(target, Target::File(_));
    let commands = crate::input::pointer::act_on(target, ui, app, seen);
    if file {
        ui.focus = Focus::Workspace;
    }
    commands
}

/// Space on a changed file: staged when it was not, unstaged when it was.
fn staged(target: &Target, app: &AppState) -> Vec<Command> {
    let Target::File(path) = target else {
        return Vec::new();
    };
    let Some(file) = app.workspace.files.iter().find(|one| &one.path == path) else {
        return Vec::new();
    };
    let path = path.clone();
    let command = match file.staged {
        Some(true) => workspace::Command::Unstage { path },
        _ => workspace::Command::Stage { path },
    };
    vec![Command::Workspace(command)]
}

/// Left shuts a folder, Right opens it, in the tree and in the changed list alike.
fn folded(target: &Target, ui: &mut Ui, open: bool) {
    let (held, path, holds_open) = match target {
        Target::Dir(path) => (&mut ui.session.opened, path, true),
        Target::Group(dir) => (&mut ui.session.closed, dir, false),
        Target::FoundIn(path) => (&mut ui.session.shut, path, false),
        _ => return,
    };
    let wanted = open == holds_open;
    let has = held.contains(path);
    if wanted != has {
        groove_types::toggle(held, path.clone());
    }
}

/// The next or the previous of the lists the tab offers.
fn paned(ui: &mut Ui, forward: bool) -> Vec<Command> {
    let offered: &[Pane] = match ui.session.tab {
        Tab::Diff => &Pane::ALL,
        _ => &[Pane::Files, Pane::Notes],
    };
    let at = offered
        .iter()
        .position(|one| *one == ui.session.pane)
        .unwrap_or(0);
    let len = offered.len();
    let next = match forward {
        true => (at + 1) % len,
        false => (at + len - 1) % len,
    };
    ui.session.pane = offered[next];
    ui.session.files = 0.0;
    ui.hover = None;
    Vec::new()
}
