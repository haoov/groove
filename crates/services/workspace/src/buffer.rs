//! The open files' buffers: what a read, a derive and a save do to them.

use groove_types::{Selection, WorktreeId};

use crate::{Buffers, Derived, Opened, State, by_line, from_documents};

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

    /// Installs what a derive found, when the buffer is still the one it read.
    pub fn derived(&mut self, (worktree, path): (&WorktreeId, &str), read: Derived, revision: u64) {
        let Some(open) = self
            .buffers
            .get_mut(worktree)
            .and_then(|one| one.get_mut(path))
        else {
            return;
        };
        if !open.new.settled(read.settled, revision) {
            return;
        }
        open.rows = read.aligned.rows.clone();
        open.marks = read.aligned.marks.clone();
        open.words = by_line(&read.aligned.rows, &read.aligned.words);
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
    open.rows = file.rows;
    open.marks = file.marks;
    open.words = file.words;
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
