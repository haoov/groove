//! What a press, a drag and a click do, through what the last frame drew.

use groove_controllers::{AppState, Command, session, workspace};
use groove_types::{DiffView, WorktreeId};

use super::Key;
use crate::ctx::Metrics;
use crate::hit::{Hits, Target};
use crate::layout::Edge;
use crate::palette::{Action, Flow, Palette};
use crate::tokens::Tokens;
use crate::views::session::components::diff;
use crate::widget::code_at;
use crate::{Drag, Focus, Ui};

/// A press on a boundary takes hold of it; anywhere else is a click.
pub(super) fn press(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    if let Some(Target::Split(edge)) = hits.at(x, y) {
        grab(ui, edge, x, metrics);
        return Vec::new();
    }
    click(x, y, ui, app, hits, metrics)
}

/// Takes hold of `edge`, keeping how far from it the pointer landed.
fn grab(ui: &mut Ui, edge: Edge, x: f32, metrics: Metrics) {
    let at = ui.split.edge_at(edge, width_of(metrics), sidebar(ui));
    ui.drag = Some(Drag {
        edge,
        offset: logical(x, metrics) - at,
    });
}

/// The boundary follows the pointer.
pub(super) fn drag_to(ui: &mut Ui, x: f32, metrics: Metrics) {
    let Some(drag) = ui.drag else {
        return;
    };
    let at = logical(x, metrics) - drag.offset;
    ui.split.drag(drag.edge, at, width_of(metrics), sidebar(ui));
}

fn sidebar(ui: &Ui) -> bool {
    ui.session.sidebar()
}

/// The window's width in logical pixels.
fn width_of(metrics: Metrics) -> f32 {
    logical(metrics.size.rect().w, metrics)
}

fn logical(value: f32, metrics: Metrics) -> f32 {
    value / metrics.scale
}

/// What was drawn under the point, acted on. Anywhere else closes the palette.
fn click(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    let target = hits.at(x, y);
    let inside = matches!(target, Some(Target::Palette | Target::PaletteRow(_)));
    if ui.palette.is_some() && !inside {
        ui.palette = None;
        return Vec::new();
    }
    ui.focus = focused(&target, ui.focus);
    match target {
        Some(Target::Session(session)) => {
            vec![Command::Session(session::Command::Select { session })]
        }
        Some(Target::Tab(tab)) => {
            ui.session.tab = tab;
            Vec::new()
        }
        Some(Target::Fold) => {
            ui.session.folded = !ui.session.folded;
            Vec::new()
        }
        Some(Target::Picker) => selector(ui, app),
        Some(Target::Worktree(worktree)) => select_worktree(app, worktree),
        Some(Target::File(path)) => {
            vec![Command::Workspace(workspace::Command::OpenFile { path })]
        }
        Some(Target::View(view)) => {
            switch(ui, app, view, metrics);
            Vec::new()
        }
        Some(Target::Code) => {
            ui.session.at = caret(ui, hits, metrics, (x, y));
            Vec::new()
        }
        Some(Target::PaletteRow(at)) => palette_row(at, ui, app),
        Some(Target::Agent | Target::Palette | Target::Split(_)) | None => Vec::new(),
    }
}

/// Changes the view, keeping the line at the top of the old one in view.
fn switch(ui: &mut Ui, app: &AppState, view: DiffView, metrics: Metrics) {
    let line = Tokens::new(metrics.scale).line;
    let from = ui.session.view;
    ui.session.diff = diff::scrolled(app, from, view, ui.session.diff, line);
    ui.session.at = ui
        .session
        .at
        .map(|(row, column)| (diff::moved(app, from, view, row), column));
    ui.session.view = view;
}

/// The pane a click lands in. What it lands on says which.
fn focused(target: &Option<Target>, focus: Focus) -> Focus {
    match target {
        Some(Target::Session(_)) => Focus::Rail,
        Some(Target::Agent) => Focus::Agent,
        Some(Target::Code) | Some(Target::View(_)) => Focus::Workspace,
        Some(Target::File(_)) => Focus::Sidebar,
        _ => focus,
    }
}

/// The row and column a click lands on in the open file.
fn caret(ui: &Ui, hits: &Hits, metrics: Metrics, point: (f32, f32)) -> Option<(usize, usize)> {
    let rect = hits.rect_of(&Target::Code)?;
    let tokens = Tokens::new(metrics.scale);
    code_at(&tokens, metrics.cell, rect, ui.session.diff, point)
}

/// Either picker opens the worktree selector.
fn selector(ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let flow = Flow::new(Action::SelectWorktree, session);
    let commands = flow.refresh(app).into_iter().collect();
    ui.palette = Some(Palette {
        flow: Some(flow),
        ..Palette::default()
    });
    commands
}

fn select_worktree(app: &AppState, worktree: WorktreeId) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    vec![Command::Session(session::Command::SelectWorktree {
        session,
        worktree,
    })]
}

/// A click on a row is that row selected, then confirmed.
fn palette_row(at: usize, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let Some(palette) = &mut ui.palette else {
        return Vec::new();
    };
    palette.selected = at;
    let outcome = palette.key(Key::Enter, app);
    if outcome.close {
        ui.palette = None;
    }
    outcome.commands
}
