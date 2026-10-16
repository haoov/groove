//! The sidebar's rows as the arrows walk them: each one's target, where it starts, how tall.

use groove_controllers::AppState;
use groove_ui_kit::base::tokens::Tokens;

use super::notes::{Item, items};
use super::{Pane, browsing, explorer, listing, narrowed};
use crate::Ui;
use crate::hit::Target;

/// One row the arrows stop on, its top in the list's own coordinates.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Stop {
    pub target: Target,
    pub top: f32,
    pub height: f32,
}

/// The rows of the list the sidebar shows, in the order it draws them.
pub(crate) fn stops(app: &AppState, ui: &Ui, tokens: &Tokens) -> Vec<Stop> {
    let mut rows = Rows::default();
    let row = tokens.row;
    match ui.session.pane {
        Pane::Commits => {
            for one in &app.workspace.log {
                rows.push(Some(Target::Commit(one.sha.clone())), row);
            }
        }
        Pane::Notes => {
            for item in items(app, ui.session.showing) {
                match item {
                    Item::Head(..) => rows.push(None, row),
                    Item::Note(at) => rows.push(Some(Target::NoteAt(at)), row + tokens.line),
                }
            }
        }
        Pane::Files if ui.session.bar.greps() => {
            for item in super::results::items(&app.workspace.found, ui) {
                let target = match item {
                    super::results::Item::File(path, _) => Target::FoundIn(path.to_string()),
                    super::results::Item::Match(at, _) => Target::Found(at),
                };
                rows.push(Some(target), row);
            }
        }
        Pane::Files if browsing(ui) => tree(&mut rows, app, ui, row),
        Pane::Files => changed(&mut rows, app, ui, row),
    }
    rows.out
}

/// The rows laid down so far, and where the next one starts.
#[derive(Default)]
struct Rows {
    top: f32,
    out: Vec<Stop>,
}

impl Rows {
    fn push(&mut self, target: Option<Target>, height: f32) {
        if let Some(target) = target {
            let top = self.top;
            self.out.push(Stop {
                target,
                top,
                height,
            });
        }
        self.top += height;
    }
}

/// The worktree's tree: each directory standing open, each file.
fn tree(rows: &mut Rows, app: &AppState, ui: &Ui, row: f32) {
    let changed = narrowed(app, ui);
    let paths = (app.workspace.paths(), app.workspace.paths_stamp());
    for one in explorer::rows(paths, &changed, ui) {
        let target = match one.dir {
            true => Target::Dir(one.path),
            false => Target::File(one.path),
        };
        rows.push(Some(target), row);
    }
}

/// The changed files under their directories, a folded directory's files left out.
fn changed(rows: &mut Rows, app: &AppState, ui: &Ui, row: f32) {
    let files = narrowed(app, ui);
    let listed = listing(&files, !ui.session.bar.path.is_empty());
    if !listed.root.is_empty() {
        rows.push(None, row);
    }
    for group in &listed.groups {
        if !group.dir.is_empty() {
            rows.push(Some(Target::Group(group.dir.clone())), row);
            if ui.session.closed.contains(&group.dir) {
                continue;
            }
        }
        for file in &group.files {
            rows.push(Some(Target::File(file.path.clone())), row);
        }
    }
}
