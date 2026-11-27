//! What a click on the sidebar's own rows does: its scope, its twisties, its bars.

use groove_controllers::{AppState, Command};

use crate::Ui;

/// One of the sidebar's lists up, or which notes its list shows; the frame reads what it shows.
pub(super) fn paned(ui: &mut Ui, target: crate::hit::Target) -> Vec<Command> {
    match target {
        crate::hit::Target::Pane(pane) => ui.session.pane = pane,
        crate::hit::Target::Showing(showing) => ui.session.showing = showing,
        _ => return Vec::new(),
    }
    ui.session.files = 0.0;
    Vec::new()
}

/// One note of the list: the Diff on its file at its line, a resolved one opened.
pub(super) fn note_at(ui: &mut Ui, app: &AppState, at: usize) -> Vec<Command> {
    let Some(note) = app.delivery.shown.get(at) else {
        return Vec::new();
    };
    let Some(anchor) = note.anchor.clone() else {
        return Vec::new();
    };
    if note.resolved && !ui.session.unfolded.contains(&note.thread) {
        ui.session.unfolded.push(note.thread.clone());
    }
    ui.focus = crate::Focus::Workspace;
    ui.session.mode = crate::views::session::Mode::Diff;
    ui.session.revealing = Some((anchor.path.clone(), anchor.start_line));
    shown(ui, anchor.path)
}

/// The Diff on one file, from its top.
pub(super) fn shown(ui: &mut Ui, path: String) -> Vec<Command> {
    (ui.session.diff, ui.session.diff_across) = (0.0, 0.0);
    let focus = groove_controllers::workspace::Command::Focus { path: Some(path) };
    vec![Command::Workspace(focus)]
}

/// A directory's twisty: the explorer's opens or shuts, the changed list's folds or opens.
pub(super) fn twisty(ui: &mut Ui, target: crate::hit::Target) -> Vec<Command> {
    let (held, path) = match target {
        crate::hit::Target::Dir(path) => (&mut ui.session.opened, path),
        crate::hit::Target::Group(dir) => (&mut ui.session.closed, dir),
        _ => return Vec::new(),
    };
    groove_types::toggle(held, path);
    Vec::new()
}

/// The keyboard back into the find bar, at the end of what it holds.
pub(super) fn finding(ui: &mut Ui) -> Vec<Command> {
    if let Some(find) = ui.session.find.as_mut() {
        find.typing = true;
        find.query.end();
    }
    Vec::new()
}

/// What a click on `target` does, from the keyboard: at the middle of where it was drawn.
pub(in crate::input) fn act_on(
    target: crate::hit::Target,
    ui: &mut crate::Ui,
    app: &groove_controllers::AppState,
    (hits, metrics): (&crate::hit::Hits, groove_ui_kit::base::ctx::Metrics),
) -> Vec<groove_controllers::Command> {
    let point = hits.rect_of(&target).map_or((0.0, 0.0), |rect| {
        (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0)
    });
    super::acted(Some(target), point, ui, app, hits, metrics)
}
