//! What a press, a drag and a click do, through what the last frame drew.

use groove_controllers::{AppState, Command, session, workspace};
use groove_types::{Caret, DiffView, Edit, Motion, WorktreeId};

use super::Key;
use crate::ctx::Metrics;
use crate::hit::{Hits, Picks, Target};
use crate::layout::{Edge, Layout};
use crate::palette::{Action, Anchor, Flow, Palette};
use crate::tokens::{CLICK_MS, CLICK_SLOP};
use crate::views::session::components::diff;
use crate::widget::code_at;
use crate::{Click, Corner, Drag, Focus, Losing, Menu, Of, Ui};
use groove_controllers::workspace_service::columns;

/// A press on a boundary takes hold of it; anywhere else is a click.
pub(super) fn press(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    ui.clicked = Some(counted(ui.clicked, x, y, metrics));
    if let Some(Target::Split(edge)) = hits.at(x, y) {
        grab(ui, edge, x, y, metrics);
        return Vec::new();
    }
    click(x, y, ui, app, hits, metrics)
}

/// The right button on a file's row opens its actions; anywhere else closes them.
pub(super) fn asked(x: f32, y: f32, ui: &mut Ui, hits: &Hits, metrics: Metrics) {
    ui.discarding = None;
    ui.menu = match hits.at(x, y) {
        Some(Target::File(path)) => Some(Menu {
            at: (x, y),
            corner: Corner::TopLeft,
            of: Of::File(path),
        }),
        Some(Target::Actions) => Some(worktree_menu(ui, hits, metrics)),
        _ => None,
    };
}

/// This press, against the one before it: a press soon after another and near it
/// carries the same click on.
fn counted(last: Option<Click>, x: f32, y: f32, metrics: Metrics) -> Click {
    let slop = CLICK_SLOP * metrics.scale;
    let same = last.filter(|last| {
        metrics.tick.saturating_sub(last.at) <= CLICK_MS
            && (last.x - x).abs() <= slop
            && (last.y - y).abs() <= slop
    });
    Click {
        x,
        y,
        at: metrics.tick,
        count: same.map_or(1, |last| last.count + 1),
    }
}

/// Takes hold of `edge`, keeping how far from it the pointer landed.
fn grab(ui: &mut Ui, edge: Edge, x: f32, y: f32, metrics: Metrics) {
    let at = ui.split.edge_at(edge, window_of(metrics), sidebar(ui));
    ui.drag = Some(Drag {
        edge,
        offset: along(edge, x, y, metrics) - at,
    });
}

/// The pointer's place along the axis the boundary moves in, in logical pixels.
fn along(edge: Edge, x: f32, y: f32, metrics: Metrics) -> f32 {
    match edge.upright() {
        true => logical(x, metrics),
        false => logical(y, metrics),
    }
}

/// The pointer moved: a boundary follows it, or the open file holds more.
pub(super) fn moved(
    x: f32,
    y: f32,
    ui: &mut Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
) -> Vec<Command> {
    if ui.drag.is_some() {
        drag_to(ui, x, y, metrics);
        return Vec::new();
    }
    if !ui.selecting {
        return Vec::new();
    }
    let Some(caret) = caret(ui, app, hits, metrics, (x, y)) else {
        return Vec::new();
    };
    let edit = Edit::Extend(Motion::To(caret));
    vec![Command::Workspace(workspace::Command::Edit(edit))]
}

/// The boundary follows the pointer.
fn drag_to(ui: &mut Ui, x: f32, y: f32, metrics: Metrics) {
    let Some(drag) = ui.drag else {
        return;
    };
    let at = along(drag.edge, x, y, metrics) - drag.offset;
    ui.split
        .drag(drag.edge, at, window_of(metrics), sidebar(ui));
}

fn sidebar(ui: &Ui) -> bool {
    ui.session.sidebar()
}

/// The window's width in logical pixels.
fn window_of(metrics: Metrics) -> (f32, f32) {
    let rect = metrics.size.rect();
    (logical(rect.w, metrics), logical(rect.h, metrics))
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
    if ui.menu.is_some() {
        return chosen(target, ui);
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
        Some(Target::Picker(which)) => selector(ui, app, hits, which),
        Some(Target::Worktree(worktree)) => select_worktree(app, worktree),
        Some(Target::File(path)) => {
            vec![Command::Workspace(workspace::Command::OpenFile { path })]
        }
        Some(Target::View(view)) => {
            switch(ui, app, view, metrics);
            Vec::new()
        }
        Some(Target::Code) => {
            ui.selecting = true;
            landed(ui, app, hits, metrics, (x, y))
        }

        Some(Target::Stage(path)) => {
            vec![Command::Workspace(workspace::Command::Stage { path })]
        }
        Some(Target::Unstage(path)) => {
            vec![Command::Workspace(workspace::Command::Unstage { path })]
        }
        Some(Target::Discard) => lose(ui),
        Some(Target::Keep) => {
            ui.discarding = None;
            Vec::new()
        }
        Some(Target::Actions) => {
            ui.menu = Some(worktree_menu(ui, hits, metrics));
            Vec::new()
        }
        Some(Target::Message) => {
            ui.session.composing = true;
            Vec::new()
        }
        Some(Target::Do) => {
            ui.session.composing = false;
            let act = crate::views::session::components::commit::primary(app);
            act.map(Command::Workspace).into_iter().collect()
        }
        Some(Target::PaletteRow(at)) => palette_row(at, ui, app),
        Some(Target::Agent | Target::Palette | Target::Split(_) | Target::MenuRow(_)) | None => {
            Vec::new()
        }
    }
}

/// The worktree's actions stand above the caret that opened them, ending on the rule
/// that separates the box from the list.
fn worktree_menu(ui: &Ui, hits: &Hits, metrics: Metrics) -> Menu {
    let box_ = Layout::of(metrics, ui).commit;
    let right = hits
        .rect_of(&Target::Actions)
        .map(|caret| caret.right())
        .unwrap_or(box_.right());
    Menu {
        at: (right, box_.y),
        corner: Corner::BottomRight,
        of: Of::Worktree,
    }
}

/// The answer that throws the change away: one file's, or every one.
fn lose(ui: &mut Ui) -> Vec<Command> {
    let asked = ui.discarding.take();
    let command = match asked {
        Some(Losing::File(path)) => workspace::Command::Discard { path },
        Some(Losing::Everything) => workspace::Command::DiscardAll,
        None => return Vec::new(),
    };
    vec![Command::Workspace(command)]
}

/// A click while a menu is open: a row of it, or anywhere to close it.
fn chosen(target: Option<Target>, ui: &mut Ui) -> Vec<Command> {
    let menu = ui.menu.take();
    let (Some(Target::MenuRow(at)), Some(menu)) = (target, menu) else {
        return Vec::new();
    };
    let (commands, asking) = crate::views::shared::actions::picked(&menu.of, at);
    ui.discarding = asking;
    commands
}

/// Changes the view, keeping the line at the top of the old one in view.
fn switch(ui: &mut Ui, app: &AppState, view: DiffView, metrics: Metrics) {
    let line = metrics.tokens().line;
    let from = ui.session.view;
    ui.session.diff = diff::scrolled(app, from, view, ui.session.diff, line);
    ui.session.view = view;
}

/// The pane a click lands in. What it lands on says which.
fn focused(target: &Option<Target>, focus: Focus) -> Focus {
    match target {
        Some(Target::Session(_)) => Focus::Rail,
        Some(Target::Agent) => Focus::Agent,
        Some(Target::Code) | Some(Target::View(_)) => Focus::Workspace,
        Some(
            Target::File(_)
            | Target::Stage(_)
            | Target::Unstage(_)
            | Target::Discard
            | Target::Keep
            | Target::Message
            | Target::Do,
        ) => Focus::Sidebar,
        _ => focus,
    }
}

/// Where a click in the open file puts the caret. A row the new side has no line
/// on — a removed one, a gap — takes no caret.
fn landed(
    ui: &Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
    point: (f32, f32),
) -> Vec<Command> {
    let Some(caret) = caret(ui, app, hits, metrics, point) else {
        return Vec::new();
    };
    let mut edits = vec![Edit::Move(Motion::To(caret))];
    edits.extend(taken(ui.clicked));
    edits
        .into_iter()
        .map(|edit| Command::Workspace(workspace::Command::Edit(edit)))
        .collect()
}

/// What a click of its own takes: one lands the caret, two a word, three the line.
fn taken(click: Option<Click>) -> Option<Edit> {
    match click?.count {
        2 => Some(Edit::SelectWord),
        count if count >= 3 => Some(Edit::SelectLine),
        _ => None,
    }
}

fn caret(
    ui: &Ui,
    app: &AppState,
    hits: &Hits,
    metrics: Metrics,
    point: (f32, f32),
) -> Option<Caret> {
    let rect = hits.rect_of(&Target::Code)?;
    let tokens = metrics.tokens();
    let (row, display) = code_at(&tokens, hits.chars(), rect, point)?;
    let line = diff::line_at(app, ui.session.view, row)?;
    let file = app.workspace.opened.as_ref()?;
    let text = file.new.line(line).unwrap_or_default();
    let width = file.new.document().indent().width();
    Some(Caret::new(line, columns(&text, display, width)))
}

/// Either picker opens the worktree selector, under the picker itself.
fn selector(ui: &mut Ui, app: &AppState, hits: &Hits, which: Picks) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let flow = Flow::new(Action::SelectWorktree, session);
    let commands = flow.refresh(app).into_iter().collect();
    ui.palette = Some(Palette {
        flow: Some(flow),
        anchor: hits.rect_of(&Target::Picker(which)).map(Anchor::under),
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
