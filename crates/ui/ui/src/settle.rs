//! What the layout reads from the state each frame, and what ends when the selection moves.

use groove_controllers::AppState;
use groove_types::SessionId;

use crate::views::session::Tab;
use crate::{Overlay, Surface, Ui};

impl Ui {
    /// Reads the layout's state, drops what a move left, lands a pending tab, and leaves Kubernetes with no cluster.
    pub fn settle(&mut self, app: &AppState) {
        let selected = app.session.selected().map(|open| &open.session.kind);
        self.session.alone = selected.is_some_and(|kind| kind.routine().is_some());
        self.session.reading = app.workspace.commit.is_some();
        self.kept_resources(app);
        self.moved(app);
        self.left(app);
        self.session.arrive(app);
        let held = app
            .session
            .selected()
            .is_some_and(|open| open.holds_clusters());
        if self.session.tab == Tab::Resources && !held {
            self.session.tab = Tab::Overview;
        }
        if !self.session.sidebar() && self.focus == crate::Focus::Sidebar {
            self.focus = crate::Focus::Workspace;
        }
        self.session.composing &= self.session.commits();
        self.unfound();
        self.refound(app);
        crate::views::session::diff::blocks::follow(self, app);
    }

    /// The find bar lets go of the keys on a tab change, and while no bar is drawn.
    fn unfound(&mut self) {
        let session = &mut self.session;
        let moved = std::mem::replace(&mut session.settled, session.tab) != session.tab;
        let hidden = moved || !session.finds();
        if let Some(find) = session.find.as_mut().filter(|_| hidden) {
            find.typing = false;
        }
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

    /// A question or a draft about the worktree, or the session, left behind, dropped; the Resources tab kept per session.
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
        let promoted = match (&was.0, &now.0) {
            (Some(from), Some(to)) => app.session.promoted.contains(&(from.clone(), to.clone())),
            _ => false,
        };
        if session_left {
            self.session.going = None;
        }
        if session_left && !promoted {
            self.swap_resources(was.0, now.0.clone());
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

    /// Each open session's Kubernetes tab under its id, a promoted explorer's under its task's.
    fn kept_resources(&mut self, app: &AppState) {
        let resources_of = &mut self.session.resources_of;
        for (from, to) in &app.session.promoted {
            if let Some(kept) = resources_of.remove(from) {
                resources_of.entry(to.clone()).or_insert(kept);
            }
        }
        resources_of.retain(|id, _| app.session.get(id).is_some());
    }

    fn swap_resources(&mut self, was: Option<SessionId>, now: Option<SessionId>) {
        let Some(was) = was else {
            return;
        };
        let session = &mut self.session;
        let mut left = std::mem::take(&mut session.resources);
        (left.typing, left.filtering) = (false, false);
        session.resources_of.insert(was, left);
        let back = now.and_then(|id| session.resources_of.remove(&id));
        session.resources = back.unwrap_or_default();
    }
}
