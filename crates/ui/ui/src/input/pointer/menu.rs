//! The menus a click opens, and what picking a row of one does.

use groove_controllers::{AppState, Command, session, workspace};
use groove_types::WorktreeId;

use crate::ctx::Metrics;
use crate::hit::{Hits, Picks, Target};
use crate::input::Key;
use crate::layout::Layout;
use crate::palette::{Action, Anchor, Flow, Palette};
use crate::{Corner, Losing, Menu, Of, Ui};

/// The right button on a file's row opens its actions; anywhere else closes them.
pub(crate) fn asked(x: f32, y: f32, ui: &mut Ui, app: &AppState, hits: &Hits, metrics: Metrics) {
    ui.discarding = None;
    ui.menu = match hits.at(x, y) {
        Some(Target::Dir(path)) => Some(Menu {
            at: (x, y),
            corner: Corner::TopLeft,
            of: Of::Path { path, dir: true },
        }),
        Some(Target::File(path)) if browsing(ui) => Some(Menu {
            at: (x, y),
            corner: Corner::TopLeft,
            of: Of::Path { path, dir: false },
        }),
        Some(Target::File(path)) => Some(Menu {
            at: (x, y),
            corner: Corner::TopLeft,
            of: Of::File(path),
        }),
        Some(Target::Actions) => Some(worktree_menu(ui, app, hits, metrics)),
        _ => None,
    };
}

/// The worktree's actions stand above the caret that opened them, ending on the rule
/// that separates the box from the list.
pub(super) fn worktree_menu(ui: &Ui, app: &AppState, hits: &Hits, metrics: Metrics) -> Menu {
    let box_ = Layout::of(metrics, ui).commit;
    let right = hits
        .rect_of(&Target::Actions)
        .map(|caret| caret.right())
        .unwrap_or(box_.right());
    Menu {
        at: (right, box_.y),
        corner: Corner::BottomRight,
        of: Of::Worktree { mr: has_mr(app) },
    }
}

/// Whether the sidebar is browsing the worktree rather than listing its change.
fn browsing(ui: &Ui) -> bool {
    crate::views::session::files::browsing(ui)
}

/// Whether the selected worktree has a merge request to write.
fn has_mr(app: &AppState) -> bool {
    app.workspace.delivery.mr.is_some()
}

/// The answer that throws the change away: one file's, or every one.
pub(super) fn lose(ui: &mut Ui) -> Vec<Command> {
    let asked = ui.discarding.take();
    let command = match asked {
        Some(Losing::File(path)) => workspace::Command::Discard { path },
        Some(Losing::Path(path)) => {
            workspace::Command::Path(groove_controllers::workspace_service::PathOp::Delete { path })
        }
        Some(Losing::Everything) => workspace::Command::DiscardAll,
        None => return Vec::new(),
    };
    vec![Command::Workspace(command)]
}

/// A click while a menu is open: a row of it, or anywhere to close it.
pub(super) fn chosen(target: Option<Target>, ui: &mut Ui) -> Vec<Command> {
    let menu = ui.menu.take();
    let (Some(Target::MenuRow(at)), Some(menu)) = (target, menu) else {
        return Vec::new();
    };
    let (commands, asking, naming) = crate::views::shared::actions::picked(&menu.of, at);
    ui.discarding = asking;
    ui.session.naming = naming;
    commands
}

/// Either picker opens the worktree selector, under the picker itself.
pub(super) fn selector(ui: &mut Ui, app: &AppState, hits: &Hits, which: Picks) -> Vec<Command> {
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

pub(super) fn select_worktree(app: &AppState, worktree: WorktreeId) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    vec![Command::Session(session::Command::SelectWorktree {
        session,
        worktree,
    })]
}

/// A click on a row is that row selected, then confirmed.
pub(super) fn palette_row(at: usize, ui: &mut Ui, app: &AppState) -> Vec<Command> {
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
