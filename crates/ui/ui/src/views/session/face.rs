//! What the surface shows, and how far it is scrolled each way.

use super::{Face, SessionUi, Tab};

impl SessionUi {
    /// What the surface shows: a buffer whole, in Files or an object's YAML, or the change.
    pub fn face(&self) -> Face {
        match self.tab {
            Tab::Files => Face::File,
            Tab::Resources if self.resources.tab().is_some_and(|one| one.view.edits()) => {
                Face::File
            }
            _ => Face::Stream(self.view),
        }
    }

    /// How far the surface the tab shows is scrolled.
    pub fn scroll(&self) -> f32 {
        match self.face() {
            Face::File if self.tab == Tab::Resources => self.resources.scroll(),
            Face::File => self.file,
            Face::Stream(_) => self.diff,
        }
    }

    pub fn scroll_mut(&mut self) -> &mut f32 {
        match self.face() {
            Face::File if self.tab == Tab::Resources => self.resources.scroll_mut(),
            Face::File => &mut self.file,
            Face::Stream(_) => &mut self.diff,
        }
    }

    /// How far the surface the tab shows is scrolled sideways.
    pub fn across(&self) -> f32 {
        match self.face() {
            Face::File if self.tab == Tab::Resources => self.resources.across(),
            Face::File => self.file_across,
            Face::Stream(_) => self.diff_across,
        }
    }

    pub fn across_mut(&mut self) -> &mut f32 {
        match self.face() {
            Face::File if self.tab == Tab::Resources => self.resources.across_mut(),
            Face::File => &mut self.file_across,
            Face::Stream(_) => &mut self.diff_across,
        }
    }
}
