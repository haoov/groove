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
    Some(match naming.asked {
        Asked::File => PathOp::made(&naming.at, name, false),
        Asked::Folder => PathOp::made(&naming.at, name, true),
        Asked::Rename => PathOp::renamed(&naming.at, name),
        Asked::Copy => PathOp::copied(&naming.at, name),
    })
}
