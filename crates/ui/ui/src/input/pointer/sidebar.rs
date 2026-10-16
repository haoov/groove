//! What a click on the sidebar's own rows does: its scope, its twisties, its bars.

use groove_controllers::{AppState, Command};

use crate::Ui;
use crate::views::session::Term;

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

/// One note of the list, which opens the line it stands on in the tab that shows it.
pub(super) fn note_at(
    ui: &mut Ui,
    app: &AppState,
    metrics: groove_ui_kit::base::ctx::Metrics,
    at: usize,
) -> Vec<Command> {
    let Some(anchor) = app
        .delivery
        .shown
        .get(at)
        .and_then(|note| note.anchor.clone())
    else {
        return Vec::new();
    };
    let line = anchor.start_line as usize;
    ui.focus = crate::Focus::Workspace;
    if let Some(note) = app.delivery.shown.get(at).filter(|one| one.resolved)
        && !ui.session.unfolded.contains(&note.origin)
    {
        ui.session.unfolded.push(note.origin.clone());
    }
    let height = metrics.tokens().line;
    let above = |row: usize| row.saturating_sub(groove_ui_kit::base::tokens::ABOVE_MATCH) as f32;
    let row = app.workspace.changes.row_of(&anchor.path, line as u32);
    if let Some(row) = row.filter(|_| ui.session.tab == crate::views::session::Tab::Diff) {
        ui.session.diff = above(row) * height;
        ui.session.diff_across = 0.0;
        return Vec::new();
    }
    ui.session.tab = crate::views::session::Tab::Files;
    ui.session.file = above(line) * height;
    ui.session.file_across = 0.0;
    let open = groove_controllers::workspace::Command::OpenFile {
        path: anchor.path,
        at: Some(groove_types::Selection::at(groove_types::Caret::new(
            line, 0,
        ))),
    };
    vec![Command::Workspace(open)]
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

/// The keyboard into one term of the sidebar's bar.
pub(super) fn narrowing(ui: &mut Ui, term: Term) -> Vec<Command> {
    ui.session.composing = false;
    ui.session.bar.focus(term);
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
