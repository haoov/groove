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

/// The same, for a worktree whose branch already has a merge request.
pub const WORKTREE_MR: [&str; 5] = [
    "push",
    "pull",
    "update mr",
    "close mr",
    "discard every change",
];

/// The actions of the session, from the header.
pub const SESSION: [&str; 1] = ["delete locally"];

pub fn rows(of: &Of) -> &'static [&'static str] {
    match of {
        Of::File(_) => &FILE,
        Of::Worktree { mr: true } => &WORKTREE_MR,
        Of::Worktree { mr: false } => &WORKTREE,
        Of::Session(_) => &SESSION,
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
        (Of::Worktree { .. }, Some(&"discard every change")) => {
            (Vec::new(), Some(Losing::Everything))
        }
        (Of::Worktree { .. }, Some(&"push")) => commanded(workspace::Command::Push),
        (Of::Worktree { .. }, Some(&"pull")) => commanded(workspace::Command::Pull),
        (Of::Worktree { .. }, Some(&"update mr")) => commanded(workspace::Command::UpdateMr),
        (Of::Worktree { .. }, Some(&"close mr")) => commanded(workspace::Command::CloseMr),
        (Of::Session(session), Some(&"delete locally")) => {
            let away = groove_controllers::task::Command::DeleteLocal {
                session: session.clone(),
            };
            (vec![Command::Task(away)], None)
        }
        _ => (Vec::new(), None),
    }
}
