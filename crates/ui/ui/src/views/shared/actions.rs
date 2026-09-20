//! What a menu offers, and where it is drawn.

use groove_controllers::{Command, workspace};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::widget::{menu, menu_size};
use crate::{Corner, Losing, Menu, Of, Ui};

/// The actions of one file.
pub const FILE: [&str; 1] = ["discard changes"];

/// The actions of the worktree, from the commit box.
pub const WORKTREE: [&str; 3] = ["push", "pull", "discard every change"];

pub fn rows(of: &Of) -> &'static [&'static str] {
    match of {
        Of::File(_) => &FILE,
        Of::Worktree => &WORKTREE,
    }
}

pub fn draw(ctx: &mut Ctx, ui: &Ui, open: &Menu) {
    let within = ctx.layout.window;
    let hovered = match ui.hover {
        Some(Target::MenuRow(at)) => Some(at),
        _ => None,
    };
    let rows = rows(&open.of);
    let (wide, tall) = menu_size(ctx, rows);
    let at = match open.corner {
        Corner::TopLeft => open.at,
        Corner::BottomRight => (open.at.0 - wide, open.at.1 - tall),
    };
    ctx.layer();
    let edge = ctx.styles.border();
    menu(ctx, at, within, rows, hovered, edge);
}

/// What picking row `at` of this menu does: a command, or a question first.
pub fn picked(of: &Of, at: usize) -> (Vec<Command>, Option<Losing>) {
    let commanded = |command| (vec![Command::Workspace(command)], None);
    match (of, rows(of).get(at)) {
        (Of::File(path), Some(&"discard changes")) => {
            (Vec::new(), Some(Losing::File(path.clone())))
        }
        (Of::Worktree, Some(&"discard every change")) => (Vec::new(), Some(Losing::Everything)),
        (Of::Worktree, Some(&"push")) => commanded(workspace::Command::Push),
        (Of::Worktree, Some(&"pull")) => commanded(workspace::Command::Pull),
        _ => (Vec::new(), None),
    }
}
