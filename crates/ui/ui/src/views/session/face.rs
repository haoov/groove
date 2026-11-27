//! What the surface shows, and how far it is scrolled each way.

use groove_types::DiffView;

use super::{Mode, SessionUi};

/// What the code surface draws: the active file, or the change in one of its views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    File,
    Stream(DiffView),
}

impl Default for Face {
    fn default() -> Self {
        Face::Stream(DiffView::default())
    }
}

impl SessionUi {
    /// What the surface shows: a buffer whole, in Files or an object's YAML, or the change.
    pub fn face(&self) -> Face {
        match self.mode {
            Mode::Files => Face::File,
            Mode::Kubernetes if self.edits_object() => Face::File,
            _ => Face::Stream(self.view),
        }
    }

    /// Whether the surface draws the find bar: over a diff's rows or the editor's.
    pub fn finds(&self) -> bool {
        match self.mode {
            Mode::Diff | Mode::Files => true,
            Mode::Kubernetes => self.edits_object(),
            Mode::Overview | Mode::Agent => false,
        }
    }

    fn edits_object(&self) -> bool {
        self.resources.tab().is_some_and(|one| one.view.edits())
    }

    /// How far the surface the mode shows is scrolled.
    pub fn scroll(&self) -> f32 {
        match self.face() {
            Face::File if self.mode == Mode::Kubernetes => self.resources.scroll(),
            Face::File => self.file,
            Face::Stream(_) => self.diff,
        }
    }

    pub fn scroll_mut(&mut self) -> &mut f32 {
        match self.face() {
            Face::File if self.mode == Mode::Kubernetes => self.resources.scroll_mut(),
            Face::File => &mut self.file,
            Face::Stream(_) => &mut self.diff,
        }
    }

    /// How far the surface the mode shows is scrolled sideways.
    pub fn across(&self) -> f32 {
        match self.face() {
            Face::File if self.mode == Mode::Kubernetes => self.resources.across(),
            Face::File => self.file_across,
            Face::Stream(_) => self.diff_across,
        }
    }

    pub fn across_mut(&mut self) -> &mut f32 {
        match self.face() {
            Face::File if self.mode == Mode::Kubernetes => self.resources.across_mut(),
            Face::File => &mut self.file_across,
            Face::Stream(_) => &mut self.diff_across,
        }
    }
}
