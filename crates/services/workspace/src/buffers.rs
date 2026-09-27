//! The files one worktree holds open for editing, in the order their tabs stand.

use crate::Opened;

#[derive(Debug, Default)]
pub struct Buffers {
    open: Vec<Opened>,
    active: Option<String>,
}

impl Buffers {
    pub fn all(&self) -> &[Opened] {
        &self.open
    }

    pub fn get(&self, path: &str) -> Option<&Opened> {
        self.open.iter().find(|one| one.path == path)
    }

    pub fn get_mut(&mut self, path: &str) -> Option<&mut Opened> {
        self.open.iter_mut().find(|one| one.path == path)
    }

    /// The one keystrokes go to.
    pub fn active(&self) -> Option<&Opened> {
        self.get(self.active.as_deref()?)
    }

    pub fn active_mut(&mut self) -> Option<&mut Opened> {
        let path = self.active.clone()?;
        self.get_mut(&path)
    }

    pub fn activate(&mut self, path: &str) {
        if self.get(path).is_some() {
            self.active = Some(path.to_string());
        }
    }

    /// Puts the file in its own tab, or in place of the one it replaces.
    pub fn install(&mut self, file: Opened) {
        match self.open.iter().position(|one| one.path == file.path) {
            Some(at) => self.open[at] = file,
            None => self.open.push(file),
        }
    }

    /// Takes the file out; the tab beside it becomes the active one.
    pub fn close(&mut self, path: &str) -> Option<Opened> {
        let at = self.open.iter().position(|one| one.path == path)?;
        let gone = self.open.remove(at);
        if self.active.as_deref() == Some(path) {
            let next = self.open.get(at).or_else(|| self.open.last());
            self.active = next.map(|one| one.path.clone());
        }
        Some(gone)
    }

    /// The paths whose buffers owe the disk.
    pub fn dirty(&self) -> Vec<String> {
        let owed = self.open.iter().filter(|one| one.new.dirty());
        owed.map(|one| one.path.clone()).collect()
    }
}
