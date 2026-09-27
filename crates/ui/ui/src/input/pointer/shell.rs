//! What a click on the manual section does: its fold, a new tab or pane, a tab, a grid.

use groove_controllers::{AppState, Command, shell};

use crate::hit::Target;
use crate::layout::{Layout, grid_in};
use crate::{Focus, Ui};
use groove_ui_kit::base::ctx::Metrics;

pub(super) fn acted(
    target: &Target,
    ui: &mut Ui,
    app: &AppState,
    metrics: Metrics,
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
        Target::Shell(id) => {
            ui.focus = Focus::Terminal;
            shell::Command::Focus { session, id: *id }
        }
        _ => return None,
    };
    Some(vec![Command::Shell(command)])
}

/// The grid each of `panes` terminals side by side holds in the open section.
fn sized(ui: &Ui, metrics: Metrics, panes: usize) -> (u16, u16) {
    let (layout, tokens) = (Layout::of(metrics, ui), metrics.tokens());
    let first = layout.shell_panes(&tokens, panes)[0];
    grid_in(first, &tokens, metrics.cell)
}
