//! The open files' tabs: one made active, and one closed by its cross or the middle button.

use groove_controllers::{AppState, Command, workspace};

use crate::Ui;
use crate::hit::{Hits, Target};
use crate::views::session::Tab;

/// The file active in the Files tab, from its top when it was not already.
pub(in crate::input) fn in_files(ui: &mut Ui, app: &AppState, path: String) -> Vec<Command> {
    ui.session.tab = Tab::Files;
    if app.workspace.active().is_none_or(|one| one.path != path) {
        ui.session.file = 0.0;
        ui.session.file_across = 0.0;
    }
    let twice = ui.clicked.is_some_and(|click| click.count >= 2);
    let keep = twice.then(|| workspace::Command::KeepFile { path: path.clone() });
    let open = workspace::Command::OpenFile { path, at: None };
    [Some(open), keep]
        .into_iter()
        .flatten()
        .map(Command::Workspace)
        .collect()
}

/// The middle button on a file's tab closes it, as its cross does.
pub(crate) fn middle(x: f32, y: f32, ui: &mut Ui, app: &AppState, hits: &Hits) -> Vec<Command> {
    if ui.overlay.is_some() {
        return Vec::new();
    }
    match hits.at(x, y) {
        Some(Target::OpenTab(path) | Target::CloseTab(path)) => closing(ui, app, path),
        _ => Vec::new(),
    }
}

/// A tab closed, or the question first when its file owes the disk.
pub(in crate::input) fn closing(ui: &mut Ui, app: &AppState, path: String) -> Vec<Command> {
    let owes = app
        .workspace
        .buffer(&path)
        .is_some_and(|one| one.new.dirty());
    if owes {
        ui.overlay = Some(crate::Overlay::Losing(crate::Losing::Tab(path)));
        return Vec::new();
    }
    vec![Command::Workspace(workspace::Command::CloseFile { path })]
}
