//! What the layout reads from the state each frame, and what ends when the selection moves.

use groove_controllers::AppState;

use crate::views::session::Tab;
use crate::{Overlay, Surface, Ui};

impl Ui {
    /// What the layout reads from the state: a routine's session selected, a commit shown.
    pub fn settle(&mut self, app: &AppState) {
        let selected = app.session.selected().map(|open| &open.session.kind);
        self.session.alone = selected.is_some_and(|kind| kind.routine().is_some());
        self.session.reading = app.workspace.commit.is_some();
        self.moved(app);
        self.left(app);
        self.session.arrive(app);
        let held = app
            .session
            .selected()
            .is_some_and(|open| !open.clusters.is_empty());
        if self.session.tab == Tab::Resources && !held {
            self.session.tab = Tab::Overview;
        }
        if !self.session.sidebar() && self.focus == crate::Focus::Sidebar {
            (self.focus, self.session.composing) = (crate::Focus::Workspace, false);
        }
        self.refound(app);
    }

    /// The find bar's matches read again once what they were read from changed.
    fn refound(&mut self, app: &AppState) {
        use crate::views::session::find::{found, read_of};
        let Some(find) = self.session.find.as_ref() else {
            return;
        };
        let now = read_of(app, self);
        if find.read == now || find.query.is_empty() {
            return;
        }
        let (view, query) = (find.view, find.query.text().to_string());
        let hits = found(app, self, view, &query);
        if let Some(find) = self.session.find.as_mut() {
            find.at = find.at.min(hits.len().saturating_sub(1));
            (find.hits, find.read) = (hits, now);
        }
    }

    /// The fields of the surface no longer shown let go of the keys.
    fn left(&mut self, app: &AppState) {
        let now = self.showing(app);
        if std::mem::replace(&mut self.shown, now) == now {
            return;
        }
        match now {
            Surface::Board => {
                let session = &mut self.session;
                (session.bar.typing, session.composing) = (None, false);
                (session.resources.typing, session.resources.filtering) = (false, false);
                if let Some(find) = session.find.as_mut() {
                    find.typing = false;
                }
                if matches!(self.overlay, Some(Overlay::Scope(_))) {
                    self.overlay = None;
                }
            }
            Surface::Session => self.board.typing = false,
        }
    }

    /// A question or a draft about the worktree, or the session, left behind, dropped.
    fn moved(&mut self, app: &AppState) {
        let session = app.session.selected.clone();
        let worktree = app.session.selected_worktree().map(|one| one.id.clone());
        let now = (session, worktree);
        let was = std::mem::replace(&mut self.session.seen, now.clone());
        if was == now {
            return;
        }
        (self.session.naming, self.session.noting) = (None, None);
        let session_left = was.0 != now.0;
        if session_left {
            self.session.going = None;
        }
        let ends = |one: &Overlay| match one {
            Overlay::Losing(_) | Overlay::Menu(_) => true,
            Overlay::Scope(_) => session_left,
            _ => false,
        };
        if self.overlay.as_ref().is_some_and(ends) {
            self.overlay = None;
        }
    }
}
