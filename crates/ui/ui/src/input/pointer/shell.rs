//! What a click on the manual section does: its fold, a new tab or pane, a tab, a grid.

use groove_controllers::{AppState, Command, shell};

use crate::hit::{Hits, Target};
use crate::input::selection::{pressed as pressed_on, shell_cell};
use crate::layout::{Layout, grid_in};
use crate::{Focus, Grid, Ui};
use groove_ui_kit::base::ctx::Metrics;

pub(super) fn acted(
    target: &Target,
    point: (f32, f32),
    ui: &mut Ui,
    app: &AppState,
    (hits, metrics): (&Hits, Metrics),
) -> Option<Vec<Command>> {
    let session = app.session.selected.clone()?;
    let command = match target {
        Target::ShellFold => {
            ui.session.manual = !ui.session.manual;
            return Some(Vec::new());
        }
        Target::ShellNew => {
            ui.session.manual = true;
            let (cols, rows) = sized(ui, metrics, 1);
            shell::Command::Open {
                session,
                cols,
                rows,
            }
        }
        Target::ShellSplit => {
            ui.session.manual = true;
            let up = app
                .shell
                .shells(&session)
                .map_or(0, |one| one.shown().len());
            let (cols, rows) = sized(ui, metrics, up + 1);
            shell::Command::Split {
                session,
                cols,
                rows,
            }
        }
        Target::ShellTab(tab) => shell::Command::SelectTab { session, tab: *tab },
        Target::ShellCloseTab(tab) => shell::Command::CloseTab { session, tab: *tab },
        Target::ShellClose(id) => shell::Command::Close { session, id: *id },
        Target::Shell(id) => return Some(pressed(ui, app, *id, point, (hits, metrics))),
        _ => return None,
    };
    Some(vec![Command::Shell(command)])
}

/// A press on a terminal: the keys go to it, and a selection begins where it landed.
fn pressed(
    ui: &mut Ui,
    app: &AppState,
    id: u64,
    point: (f32, f32),
    (hits, metrics): (&Hits, Metrics),
) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    ui.focus = Focus::Terminal;
    let mut out = vec![Command::Shell(shell::Command::Focus { session, id })];
    if let Some(cell) = shell_cell(id, point, hits, metrics) {
        out.extend(pressed_on(ui, app, Grid::Shell(id), cell));
    }
    out
}

/// The grid each of `panes` terminals side by side holds in the open section.
fn sized(ui: &Ui, metrics: Metrics, panes: usize) -> (u16, u16) {
    let (layout, tokens) = (Layout::of(metrics, ui), metrics.tokens());
    let first = layout.shell_panes(&tokens, panes)[0];
    grid_in(first, &tokens, metrics.cell)
}
