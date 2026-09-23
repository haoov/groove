//! What a menu offers, and where it is drawn.

use groove_controllers::workspace::Say;
use groove_controllers::{Command, workspace};
use groove_types::ReviewVerdict;

use crate::ctx::Ctx;
use crate::hit::Target;
use crate::views::session::{Asked, Naming, Noting};
use crate::widget::{menu, menu_size};
use crate::{Corner, Losing, Menu, Of, Ui};

/// The actions of one file.
pub const FILE: [&str; 1] = ["discard changes"];

/// What the lines under a click offer.
pub const LINE: [&str; 1] = ["note"];

/// The actions of the worktree, from the commit box.
pub const WORKTREE: [&str; 3] = ["push", "pull", "discard every change"];

/// The same, for a worktree whose branch already has a merge request.
pub const WORKTREE_MR: [&str; 6] = [
    "push",
    "pull",
    "update mr",
    "comment",
    "close mr",
    "discard every change",
];

/// What a session reviewing someone else's merge request offers.
pub const WORKTREE_REVIEW: [&str; 6] = [
    "pull",
    "comment",
    "approve",
    "request changes",
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
        Of::Line { .. } => &LINE,
        Of::Path { .. } => &PATH,
        Of::Worktree { review: true, .. } => &WORKTREE_REVIEW,
        Of::Worktree { mr: true, .. } => &WORKTREE_MR,
        Of::Worktree { mr: false, .. } => &WORKTREE,
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

/// What picking a row of a menu leaves: commands to send, and what the surface
/// now asks of the user.
#[derive(Debug, Default, PartialEq)]
pub struct Picked {
    pub commands: Vec<Command>,
    pub asking: Option<Losing>,
    pub naming: Option<Naming>,
    pub noting: Option<Noting>,
}

impl Picked {
    fn sends(command: workspace::Command) -> Self {
        Self {
            commands: vec![Command::Workspace(command)],
            ..Self::default()
        }
    }

    fn asks(losing: Losing) -> Self {
        Self {
            asking: Some(losing),
            ..Self::default()
        }
    }

    fn names(naming: Naming) -> Self {
        Self {
            naming: Some(naming),
            ..Self::default()
        }
    }

    fn notes(anchor: groove_types::Anchor) -> Self {
        Self {
            noting: Some(Noting::new(anchor)),
            ..Self::default()
        }
    }
}

/// What picking row `at` of this menu does: a command, a question, or words to type.
pub fn picked(of: &Of, at: usize) -> Picked {
    match (of, rows(of).get(at)) {
        (Of::File(path), Some(&"discard changes")) => Picked::asks(Losing::File(path.clone())),
        (Of::Line { path, lines }, Some(&"note")) => Picked::notes(groove_types::Anchor {
            path: path.clone(),
            start_line: lines.0,
            end_line: lines.1,
        }),
        (Of::Worktree { .. }, Some(&"discard every change")) => Picked::asks(Losing::Everything),
        (Of::Worktree { .. }, Some(&"push")) => Picked::sends(workspace::Command::Push),
        (Of::Worktree { .. }, Some(&"pull")) => Picked::sends(workspace::Command::Pull),
        (Of::Worktree { .. }, Some(&"update mr")) => Picked::sends(workspace::Command::UpdateMr),
        (Of::Worktree { .. }, Some(&"close mr")) => Picked::sends(workspace::Command::CloseMr),
        (Of::Worktree { .. }, Some(&"comment")) => {
            Picked::sends(workspace::Command::Say(Say::Comment))
        }
        (Of::Worktree { .. }, Some(&"approve")) => {
            Picked::sends(workspace::Command::Say(Say::Review(ReviewVerdict::Approve)))
        }
        (Of::Worktree { .. }, Some(&"request changes")) => Picked::sends(workspace::Command::Say(
            Say::Review(ReviewVerdict::RequestChanges),
        )),
        (Of::Path { path, dir }, Some(&"new file")) => {
            Picked::names(named(Asked::File, path, *dir, ""))
        }
        (Of::Path { path, dir }, Some(&"new directory")) => {
            Picked::names(named(Asked::Folder, path, *dir, ""))
        }
        (Of::Path { path, .. }, Some(&"rename")) => {
            Picked::names(Naming::new(Asked::Rename, path, name_of(path)))
        }
        (Of::Path { path, .. }, Some(&"copy")) => {
            Picked::names(Naming::new(Asked::Copy, path, name_of(path)))
        }
        (Of::Path { path, .. }, Some(&"delete")) => Picked::asks(Losing::Path(path.clone())),
        (Of::Session(session), Some(&"delete locally")) => {
            let away = groove_controllers::session::Command::DeleteLocal {
                session: session.clone(),
            };
            Picked {
                commands: vec![Command::Session(away)],
                ..Picked::default()
            }
        }
        _ => Picked::default(),
    }
}
