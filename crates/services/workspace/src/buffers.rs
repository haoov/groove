//! The files one worktree holds open for editing, in the order their tabs stand.

use crate::Opened;

#[derive(Debug, Default)]
pub struct Buffers {
    open: Vec<Opened>,
    active: Option<String>,
    /// The one tab the next file opened takes the place of, until it is kept.
    preview: Option<String>,
    /// A file kept before its read came back.
    kept: Option<String>,
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

    /// Puts the file in its tab; a new one is the preview, in the old preview's place.
    pub fn install(&mut self, file: Opened) {
        if let Some(at) = self.open.iter().position(|one| one.path == file.path) {
            self.open[at] = file;
            return;
        }
        let path = file.path.clone();
        match self.preview.take().and_then(|old| self.position(&old)) {
            Some(at) if !self.open[at].new.dirty() => {
                if self.active.as_deref() == Some(self.open[at].path.as_str()) {
                    self.active = Some(path.clone());
                }
                self.open[at] = file;
            }
            _ => self.open.push(file),
        }
        if self.kept.take_if(|kept| *kept == path).is_none() {
            self.preview = Some(path);
        }
    }

    /// The file stays in its tab: it is no longer the preview, or will not be once it is read.
    pub fn keep(&mut self, path: &str) {
        match self.get(path) {
            Some(_) if self.preview.as_deref() == Some(path) => self.preview = None,
            Some(_) => {}
            None => self.kept = Some(path.to_string()),
        }
    }

    /// Whether the next file opened takes this one's tab.
    pub fn previews(&self, path: &str) -> bool {
        self.preview.as_deref() == Some(path)
    }

    fn position(&self, path: &str) -> Option<usize> {
        self.open.iter().position(|one| one.path == path)
    }

    /// Takes the file out; the tab beside it becomes the active one.
    pub fn close(&mut self, path: &str) -> Option<Opened> {
        let at = self.position(path)?;
        if self.preview.as_deref() == Some(path) {
            self.preview = None;
        }
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
