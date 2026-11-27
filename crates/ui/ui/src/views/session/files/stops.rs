//! The sidebar's rows as the arrows walk them: each one's target, where it starts, how tall.

use groove_controllers::AppState;
use groove_ui_kit::base::tokens::Tokens;

use super::Pane;
use super::notes::{Item, items};
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
    if ui.session.mode == crate::views::session::Mode::Files {
        for one in super::moded::browsed::lines(app, ui) {
            rows.push(one.target(), row);
        }
        return rows.out;
    }
    if super::moded::kubing(ui) {
        for one in crate::views::session::resources::moded::aside::lines(app, ui) {
            rows.push(one.target(), row);
        }
        return rows.out;
    }
    if !super::moded::diffing(ui) {
        return Vec::new();
    }
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
        Pane::Files => moded(&mut rows, app, ui, row),
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

/// The Diff's changed files, as its tree or its flat list draws them.
fn moded(rows: &mut Rows, app: &AppState, ui: &Ui, row: f32) {
    let files: Vec<&groove_types::FileDiff> = super::changed(app).iter().collect();
    for one in super::moded::tree::lines(&files, ui.session.flat, &ui.session.closed) {
        let target = match one.kind {
            super::moded::tree::Kind::Dir(..) => Target::Group(one.path),
            super::moded::tree::Kind::File(file, _) => Target::File(file.path.clone()),
        };
        rows.push(Some(target), row);
    }
}
