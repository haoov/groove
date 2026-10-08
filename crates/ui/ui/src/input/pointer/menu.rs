//! The menus a click opens, and what picking a row of one does.

use groove_controllers::{AppState, Command, session, workspace};
use groove_types::WorktreeId;

use crate::hit::{Hits, Picks, Target};
use crate::input::Key;
use crate::layout::Layout;
use crate::palette::{Action, Anchor, Flow, Palette};
use crate::{Corner, Losing, Menu, Of, Overlay, Ui};
use groove_ui_kit::base::ctx::Metrics;

/// The right button on a file's row opens its actions; anywhere else closes them.
pub(crate) fn asked(x: f32, y: f32, ui: &mut Ui, app: &AppState, hits: &Hits, metrics: Metrics) {
    let menu = match hits.at(x, y) {
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
        Some(Target::OpenTab(path) | Target::CloseTab(path)) => Some(Menu {
            at: (x, y),
            corner: Corner::TopLeft,
            of: tab_of(app, path),
        }),
        Some(Target::Actions) => Some(worktree_menu(ui, app, hits, metrics)),
        Some(Target::Code) => lines(ui, app, hits, metrics, (x, y)).map(|of| Menu {
            at: (x, y),
            corner: Corner::TopLeft,
            of,
        }),
        _ => None,
    };
    ui.overlay = menu.map(Overlay::Menu);
}

/// What a tab's menu acts on: the tab, and the others it may close without losing edits.
fn tab_of(app: &AppState, path: String) -> Of {
    let open = app
        .workspace
        .buffers()
        .map(|one| one.all())
        .unwrap_or_default();
    let owes = open.iter().any(|one| one.path == path && one.new.dirty());
    let others = open
        .iter()
        .filter(|one| one.path != path && !one.new.dirty())
        .map(|one| one.path.clone())
        .collect();
    Of::Tab { path, owes, others }
}

/// The lines a note would stand on: the selection under the click, else its line.
fn lines(ui: &Ui, app: &AppState, hits: &Hits, metrics: Metrics, at: (f32, f32)) -> Option<Of> {
    let (path, caret) = super::surface::at(ui, app, hits, metrics, at)?;
    let line = caret.line as u32;
    let held = selected(app, &path).filter(|(from, to)| (*from..=*to).contains(&line));
    let lines = held.unwrap_or((line, line));
    let noted = app
        .delivery
        .shown
        .iter()
        .filter(|note| note.is_local() && !note.resolved)
        .any(|note| note.over(&path, lines));
    (!noted).then_some(Of::Line { path, lines })
}

/// The lines a selection of the open file covers.
fn selected(app: &AppState, path: &str) -> Option<(u32, u32)> {
    let open = app.workspace.active().filter(|one| one.path == path)?;
    let one = open.new.selections().iter().find(|one| !one.is_empty())?;
    let (from, to) = one.ends();
    Some((from.line as u32, to.line as u32))
}

/// The worktree's actions, above the caret that opened them.
pub(super) fn worktree_menu(ui: &Ui, app: &AppState, hits: &Hits, metrics: Metrics) -> Menu {
    let box_ = Layout::of(metrics, ui).commit;
    let right = hits
        .rect_of(&Target::Actions)
        .map(|caret| caret.right())
        .unwrap_or(box_.right());
    Menu {
        at: (right, box_.y),
        corner: Corner::BottomRight,
        of: Of::Worktree {
            mr: has_mr(app),
            review: reviews(app),
        },
    }
}

/// Whether the sidebar is browsing the worktree rather than listing its change.
fn browsing(ui: &Ui) -> bool {
    crate::views::session::files::browsing(ui)
}

/// Whether the selected worktree has a merge request to write.
fn has_mr(app: &AppState) -> bool {
    app.session
        .selected_worktree()
        .is_some_and(|worktree| app.delivery.has_mr(&worktree.id))
}

/// Whether the session looks at someone else's merge request.
fn reviews(app: &AppState) -> bool {
    app.session
        .selected()
        .is_some_and(|open| matches!(open.session.kind, groove_types::SessionKind::Review { .. }))
}

/// The answer that throws the change away: one file's, or every one.
pub(super) fn lose(ui: &mut Ui) -> Vec<Command> {
    let asked = match ui.close(|one| matches!(one, Overlay::Losing(_))) {
        Some(Overlay::Losing(one)) => Some(one),
        _ => None,
    };
    let command = match asked {
        Some(Losing::File(path)) => workspace::Command::Discard { path },
        Some(Losing::Tab(path)) => workspace::Command::CloseFile { path },
        Some(Losing::Path(path)) => {
            workspace::Command::Path(groove_controllers::workspace_service::PathOp::Delete { path })
        }
        Some(Losing::Everything) => workspace::Command::DiscardAll,
        None => return Vec::new(),
    };
    vec![Command::Workspace(command)]
}

/// A click while a menu is open: a row of it, or anywhere to close it.
pub(super) fn chosen(target: Option<Target>, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if matches!(ui.overlay, Some(Overlay::Scope(_))) {
        return super::resources::scoped(target, ui, app);
    }
    let menu = match ui.close(|one| matches!(one, Overlay::Menu(_))) {
        Some(Overlay::Menu(one)) => Some(one),
        _ => None,
    };
    let (Some(Target::MenuRow(at)), Some(menu)) = (target, menu) else {
        return Vec::new();
    };
    let picked = crate::views::overlays::actions::picked(&menu.of, at);
    ui.overlay = picked.asking.map(Overlay::Losing);
    ui.session.naming = picked.naming;
    ui.session.noting = picked.noting;
    picked.commands
}

/// Either picker opens the worktree selector, under the picker itself.
pub(super) fn selector(ui: &mut Ui, app: &AppState, hits: &Hits, which: Picks) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let flow = Flow::new(Action::SelectWorktree, session);
    let commands = flow.refresh(app).into_iter().collect();
    ui.overlay = Some(Overlay::Palette(Palette {
        flow: Some(flow),
        anchor: hits.rect_of(&Target::Picker(which)).map(Anchor::under),
        ..Palette::default()
    }));
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
    let Some(palette) = ui.palette_mut() else {
        return Vec::new();
    };
    palette.selected = at;
    let outcome = palette.key(Key::Enter, app);
    ui.closed_palette(outcome, app)
}
