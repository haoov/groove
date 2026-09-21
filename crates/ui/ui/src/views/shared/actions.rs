//! What a menu offers, and where it is drawn.

use groove_controllers::{Command, workspace};

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::{Asked, Naming};
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

/// The actions of one path of the explorer.
pub const PATH: [&str; 5] = ["new file", "new directory", "rename", "copy", "delete"];

/// The actions of the session, from the header.
pub const SESSION: [&str; 1] = ["delete locally"];

pub fn rows(of: &Of) -> &'static [&'static str] {
    match of {
        Of::File(_) => &FILE,
        Of::Path { .. } => &PATH,
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

/// The directory a new path goes in: the one clicked, or the one its file stands in.
fn named(asked: Asked, path: &str, dir: bool, from: &str) -> Naming {
    let at = match dir {
        true => path.to_string(),
        false => path
            .rsplit_once('/')
            .map(|(up, _)| up)
            .unwrap_or("")
            .to_string(),
    };
    Naming::new(asked, &at, from)
}

fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// What picking row `at` of this menu does: a command, a question, or a name to type.
pub fn picked(of: &Of, at: usize) -> (Vec<Command>, Option<Losing>, Option<Naming>) {
    let commanded = |command| (vec![Command::Workspace(command)], None, None);
    match (of, rows(of).get(at)) {
        (Of::File(path), Some(&"discard changes")) => {
            (Vec::new(), Some(Losing::File(path.clone())), None)
        }
        (Of::Worktree { .. }, Some(&"discard every change")) => {
            (Vec::new(), Some(Losing::Everything), None)
        }
        (Of::Worktree { .. }, Some(&"push")) => commanded(workspace::Command::Push),
        (Of::Worktree { .. }, Some(&"pull")) => commanded(workspace::Command::Pull),
        (Of::Worktree { .. }, Some(&"update mr")) => commanded(workspace::Command::UpdateMr),
        (Of::Worktree { .. }, Some(&"close mr")) => commanded(workspace::Command::CloseMr),
        (Of::Path { path, dir }, Some(&"new file")) => {
            (Vec::new(), None, Some(named(Asked::File, path, *dir, "")))
        }
        (Of::Path { path, dir }, Some(&"new directory")) => {
            (Vec::new(), None, Some(named(Asked::Folder, path, *dir, "")))
        }
        (Of::Path { path, .. }, Some(&"rename")) => (
            Vec::new(),
            None,
            Some(Naming::new(Asked::Rename, path, name_of(path))),
        ),
        (Of::Path { path, .. }, Some(&"copy")) => (
            Vec::new(),
            None,
            Some(Naming::new(Asked::Copy, path, name_of(path))),
        ),
        (Of::Path { path, .. }, Some(&"delete")) => {
            (Vec::new(), Some(Losing::Path(path.clone())), None)
        }
        (Of::Session(session), Some(&"delete locally")) => {
            let away = groove_controllers::task::Command::DeleteLocal {
                session: session.clone(),
            };
            (vec![Command::Task(away)], None, None)
        }
        _ => (Vec::new(), None, None),
    }
}
