//! The name being typed in the tree: what it becomes, and what takes it back.

use groove_controllers::workspace_service::PathOp;
use groove_controllers::{Command, workspace};

use super::super::{Key, Modifiers};
use super::typing;
use crate::Ui;
use crate::views::session::{Asked, Naming};

/// One keystroke while the tree is asking for a name.
pub(super) fn in_name(key: Key, mods: Modifiers, ui: &mut Ui) -> Vec<Command> {
    let Some(naming) = ui.session.naming.as_mut() else {
        return Vec::new();
    };
    match key {
        Key::Escape => ui.session.naming = None,
        Key::Enter => {
            let asked = op_of(naming);
            ui.session.naming = None;
            return asked
                .map(|op| vec![Command::Workspace(workspace::Command::Path(op))])
                .unwrap_or_default();
        }
        key => {
            typing(key, mods, &mut naming.field);
        }
    }
    Vec::new()
}

/// What the name it holds asks of the worktree, or nothing while it is empty.
fn op_of(naming: &Naming) -> Option<PathOp> {
    let name = naming.named();
    if name.is_empty() {
        return None;
    }
    match naming.asked {
        Asked::File => Some(PathOp::Create {
            path: joined(&naming.at, name),
            folder: false,
        }),
        Asked::Folder => Some(PathOp::Create {
            path: joined(&naming.at, name),
            folder: true,
        }),
        Asked::Rename => Some(PathOp::Rename {
            from: naming.at.clone(),
            to: beside(&naming.at, name),
        }),
        Asked::Copy => Some(PathOp::Copy {
            from: naming.at.clone(),
            to: beside(&naming.at, name),
        }),
    }
}

/// A name inside a directory, which may be the worktree root itself.
fn joined(dir: &str, name: &str) -> String {
    match dir.is_empty() {
        true => name.to_string(),
        false => format!("{dir}/{name}"),
    }
}

/// A name in the same directory as the path it is given to.
fn beside(path: &str, name: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/{name}"),
        None => name.to_string(),
    }
}
