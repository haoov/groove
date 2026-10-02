//! The open files' buffers: what a read, a derive and a save do to them.

use groove_types::{Edit, Selection, WorktreeId};

use crate::{Buffers, Derived, Opened, State, from_documents};

impl State {
    pub fn buffers(&self) -> Option<&Buffers> {
        self.buffers.get(self.worktree.as_ref()?)
    }

    fn buffers_mut(&mut self) -> Option<&mut Buffers> {
        let worktree = self.worktree.clone()?;
        Some(self.buffers.entry(worktree).or_default())
    }

    /// The file keystrokes go to.
    pub fn active(&self) -> Option<&Opened> {
        self.buffers()?.active()
    }

    pub fn active_mut(&mut self) -> Option<&mut Opened> {
        self.buffers_mut()?.active_mut()
    }

    pub fn buffer(&self, path: &str) -> Option<&Opened> {
        self.buffers()?.get(path)
    }

    /// A worktree gone, and every file it held open with it.
    pub fn forget(&mut self, worktree: &WorktreeId) {
        self.buffers.remove(worktree);
    }

    /// One edit of the active file, its hunks moved along it at once. Returns the file when its text changed.
    pub fn edit(&mut self, edit: &Edit) -> Option<String> {
        let stream = self.commit.is_none();
        let open = self.active_mut()?;
        let touched = open.new.edit(edit);
        if touched.is_empty() {
            return None;
        }
        for one in &touched {
            open.edited(*one);
        }
        let (path, new) = (open.path.clone(), open.new.document().clone());
        self.keep(&path);
        if stream {
            for one in touched {
                self.changes.edited(&path, one, &new);
            }
        }
        Some(path)
    }

    /// Installs what a derive of `revision` found: its tree always, its rows while nothing was typed since.
    pub fn derived(&mut self, (worktree, path): (&WorktreeId, &str), read: Derived, revision: u64) {
        let Some(open) = self
            .buffers
            .get_mut(worktree)
            .and_then(|one| one.get_mut(path))
        else {
            return;
        };
        if !open.new.settled(read.settled) || open.new.revision() != revision {
            return;
        }
        open.take(&read.aligned);
        if self.holds(worktree) && self.commit.is_none() {
            self.changes.replace(read.aligned);
        }
    }

    /// The buffer marked as what the disk holds.
    pub fn saved(&mut self, worktree: &WorktreeId, path: &str) {
        let held = self
            .buffers
            .get_mut(worktree)
            .and_then(|one| one.get_mut(path));
        if let Some(open) = held {
            open.new.saved();
        }
    }

    /// The two sides of a file the stream holds, aligned again for the buffer.
    pub fn in_hand(&self, path: &str) -> Option<Opened> {
        if self.commit.is_some() {
            return None;
        }
        let painted = self.coloured.get(path)?;
        Some(from_documents(
            path,
            painted.old.clone(),
            painted.new.clone(),
        ))
    }

    /// The file read, with the caret it had while it was the same file; `focus` makes it active.
    pub fn arrived(
        &mut self,
        worktree: &WorktreeId,
        file: Opened,
        at: Option<Selection>,
        focus: bool,
    ) {
        let path = file.path.clone();
        let buffers = self.buffers.entry(worktree.clone()).or_default();
        match buffers.get_mut(&path) {
            Some(open) if open.new.text() == file.new.text() => refreshed(open, file, at),
            held => {
                let caret = held.map(|open| Selection::at(open.new.caret()));
                replaced(buffers, file, at.or(caret));
            }
        }
        if focus {
            buffers.activate(&path);
        }
        self.moved();
    }

    /// An open file made active, its caret where the asking wants it.
    pub fn focus(&mut self, path: &str, at: Option<Selection>) {
        let Some(buffers) = self.buffers_mut() else {
            return;
        };
        buffers.activate(path);
        if let (Some(open), Some(held)) = (buffers.get_mut(path), at) {
            open.new.holding(held);
        }
    }

    /// The file stays in its tab instead of being the preview.
    pub fn keep(&mut self, path: &str) {
        if let Some(buffers) = self.buffers_mut() {
            buffers.keep(path);
        }
    }

    /// Takes a file's tab away, unsaved edits and all; the caller asked first.
    pub fn close(&mut self, path: &str) {
        if let Some(buffers) = self.buffers_mut() {
            buffers.close(path);
        }
        self.moved();
    }
}

/// The read brought back the text the buffer holds: the buffer stays, with its history.
fn refreshed(open: &mut Opened, file: Opened, at: Option<Selection>) {
    open.old = file.old;
    open.hunked = file.hunked;
    open.long = file.long;
    open.new.saved();
    if let Some(held) = at {
        open.new.holding(held);
    }
}

/// A different text: the buffer gives way, keeping only where the caret was.
fn replaced(buffers: &mut Buffers, mut file: Opened, at: Option<Selection>) {
    if let Some(held) = at {
        file.new.holding(held);
    }
    buffers.install(file);
}
