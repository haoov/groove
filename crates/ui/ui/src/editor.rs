//! The one editor: the buffer the surface shows whole, how it is read, and where its edits go.

use groove_controllers::workspace_service::{Buffer, Opened};
use groove_controllers::{AppState, Command, workspace};
use groove_types::Edit;

use crate::Ui;

/// Which buffer an editor shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Editing {
    /// The file open in the workspace.
    File,
}

/// One buffer as the editor reads it, with the worktree's file behind it when there is one.
pub struct Editor<'a> {
    /// What the colours are kept under, and what a search names.
    pub path: &'a str,
    pub buffer: &'a Buffer,
    /// The file's change, notes and blame, which only a worktree's file has.
    pub file: Option<&'a Opened>,
    /// What else the colours depend on, beside the buffer's own text and tree.
    pub stamp: u64,
}

impl Editing {
    /// The buffer the keyboard edits: the open file, wherever the workspace stands.
    pub fn keyed(app: &AppState, _: &Ui) -> Option<Editing> {
        app.workspace.active().map(|_| Editing::File)
    }

    pub fn editor<'a>(&self, app: &'a AppState) -> Option<Editor<'a>> {
        match self {
            Editing::File => app.workspace.active().map(|file| Editor {
                path: &file.path,
                buffer: &file.new,
                file: Some(file),
                stamp: app.workspace.stamp,
            }),
        }
    }

    /// A caret move, a change or an undo, to the buffer's owner.
    pub fn edit(&self, edit: Edit) -> Command {
        match self {
            Editing::File => Command::Workspace(workspace::Command::Edit(edit)),
        }
    }

    /// What the caret holds, to the clipboard; and cut out of the buffer with `cut`.
    pub fn copy(&self, cut: bool) -> Command {
        match (self, cut) {
            (Editing::File, false) => Command::Workspace(workspace::Command::Copy),
            (Editing::File, true) => Command::Workspace(workspace::Command::Cut),
        }
    }

    pub fn paste(&self) -> Command {
        match self {
            Editing::File => Command::Workspace(workspace::Command::Paste),
        }
    }

    pub fn save(&self) -> Option<Command> {
        match self {
            Editing::File => Some(Command::Workspace(workspace::Command::SaveFile)),
        }
    }
}
