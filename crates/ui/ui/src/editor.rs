//! The one editor: the buffer the surface shows whole, how it is read, and where its edits go.

use std::borrow::Cow;

use groove_controllers::workspace_service::{Buffer, Opened};
use groove_controllers::{AppState, Command, cluster, workspace};
use groove_types::{Edit, FollowKey};

use crate::Ui;
use crate::views::session::Tab;

/// Which buffer an editor shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Editing {
    /// The file open in the workspace.
    File,
    /// An object's YAML, read-only until its context allows writes.
    Object(FollowKey),
}

/// One buffer as the editor reads it, with the worktree's file behind it when there is one.
pub struct Editor<'a> {
    /// What the colours are kept under, and what a search names.
    pub path: Cow<'a, str>,
    pub buffer: &'a Buffer,
    /// The file's change, notes and blame, which only a worktree's file has.
    pub file: Option<&'a Opened>,
    /// What else the colours depend on, beside the buffer's own text and tree.
    pub stamp: u64,
}

impl Editing {
    /// The buffer the keyboard edits: an object's YAML while its tab shows it, else the open file.
    pub fn keyed(app: &AppState, ui: &Ui) -> Option<Editing> {
        if ui.session.tab == Tab::Resources {
            let tab = ui.session.resources.tab().filter(|one| one.yaml)?;
            return Some(Editing::Object(tab.link.key()));
        }
        app.workspace.active().map(|_| Editing::File)
    }

    pub fn editor<'a>(&self, app: &'a AppState) -> Option<Editor<'a>> {
        match self {
            Editing::File => app.workspace.active().map(|file| Editor {
                path: Cow::Borrowed(file.path.as_str()),
                buffer: &file.new,
                file: Some(file),
                stamp: app.workspace.stamp,
            }),
            Editing::Object(key) => {
                let buffer = app.cluster.store.follows.yamls.get(key)?;
                let namespace = key.namespace.as_deref().unwrap_or("-");
                let name = key
                    .fields
                    .as_deref()
                    .unwrap_or_default()
                    .trim_start_matches("metadata.name=");
                let path = format!(
                    "{}/{namespace}/{}/{name}.yaml",
                    key.context, key.kind.plural
                );
                Some(Editor {
                    path: Cow::Owned(path),
                    buffer,
                    file: None,
                    stamp: 0,
                })
            }
        }
    }

    /// A caret move, a change or an undo, to the buffer's owner.
    pub fn edit(&self, edit: Edit) -> Command {
        match self {
            Editing::File => Command::Workspace(workspace::Command::Edit(edit)),
            Editing::Object(key) => Command::Cluster(cluster::Command::Caret {
                key: Box::new(key.clone()),
                edit,
            }),
        }
    }

    /// What the caret holds, to the clipboard; and cut out of the buffer with `cut`, where it writes.
    pub fn copy(&self, cut: bool) -> Command {
        match (self, cut) {
            (Editing::File, false) => Command::Workspace(workspace::Command::Copy),
            (Editing::File, true) => Command::Workspace(workspace::Command::Cut),
            (Editing::Object(key), _) => Command::Cluster(cluster::Command::Copy {
                key: Box::new(key.clone()),
            }),
        }
    }

    pub fn paste(&self) -> Option<Command> {
        match self {
            Editing::File => Some(Command::Workspace(workspace::Command::Paste)),
            Editing::Object(_) => None,
        }
    }

    pub fn save(&self) -> Option<Command> {
        match self {
            Editing::File => Some(Command::Workspace(workspace::Command::SaveFile)),
            Editing::Object(_) => None,
        }
    }

    /// Whether a click may land the caret: an object's always, a file's unless the workspace is read-only.
    pub fn lands(&self, app: &AppState) -> bool {
        match self {
            Editing::File => !app.workspace.readonly(),
            Editing::Object(_) => true,
        }
    }
}
