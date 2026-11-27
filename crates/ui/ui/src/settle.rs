//! What the layout reads from the state each frame, and what ends when the selection moves.

use groove_controllers::AppState;
use groove_types::SessionId;

use crate::views::session::{Mode, Tab};
use crate::{Overlay, Surface, Ui};

impl Ui {
    /// Reads the layout's state, drops what a move left, lands a pending tab, and leaves Kubernetes with no cluster.
    pub fn settle(&mut self, app: &AppState) {
        let maintained = app
            .session
            .selected()
            .is_some_and(|open| open.state.maintenance);
        self.alerts.shown = maintained;
        self.modes = app.config.modes();
        let shells = app
            .session
            .selected
            .as_ref()
            .and_then(|one| app.shell.shells(one));
        self.session.terminals = shells.is_some_and(|one| !one.tabs.is_empty());
        if self.focus == crate::Focus::Alerts && !self.alerts.shown {
            self.focus = crate::Focus::Agent;
        }
        self.session.reading = app.workspace.commit.is_some();
        self.review(app);
        self.moded(app);
        self.sheeted(app);
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
        let working = matches!(self.focus, crate::Focus::Workspace | crate::Focus::Sidebar);
        if self.session.tab != self.session.settled || working {
            self.session.stowed = false;
        }
        let aside = match self.modes {
            true => !self.session.folded,
            false => self.session.sidebar(),
        };
        if !aside && self.focus == crate::Focus::Sidebar {
            self.focus = crate::Focus::Workspace;
        }
        self.unfound();
        self.refound(app);
        crate::views::session::diff::blocks::follow(self, app);
    }

    /// A session waiting on a cluster write stands on the Diff tab until it is answered, then goes back.
    fn review(&mut self, app: &AppState) {
        let Some(selected) = app.session.selected.clone() else {
            return;
        };
        let activity = app.agent.activity(&selected);
        let asks = activity.map(|one| one.asks.as_slice()).unwrap_or_default();
        let waiting = asks
            .iter()
            .find(|one| one.reviewed)
            .map(|one| one.id.clone());
        self.session.reviewing = waiting.is_some();
        let held = self.session.left.as_ref();
        let left = held.filter(|(one, ..)| *one == selected);
        let Some(id) = waiting else {
            if let Some((_, tab, mode)) = left {
                (self.session.tab, self.session.mode) = (*tab, *mode);
                self.session.left = None;
            }
            return;
        };
        if left.is_none() {
            self.session.left = Some((selected, self.session.tab, self.session.mode));
        }
        (self.session.tab, self.session.stowed) = (Tab::Diff, false);
        if self.modes {
            self.session.mode = Mode::Diff;
        }
        self.close(|one| matches!(one, Overlay::Examining(asked) if *asked == id));
    }

    /// With modes, the tab follows the mode; a file or an object opened elsewhere brings its mode up.
    fn moded(&mut self, app: &AppState) {
        if !self.modes {
            return;
        }
        let held = app
            .session
            .selected()
            .is_some_and(|one| one.holds_clusters());
        let session = &mut self.session;
        let opened = session.tab == Tab::Files && session.settled != Tab::Files;
        if opened && session.mode != Mode::Diff {
            session.mode = Mode::Files;
        }
        let kubed = session.tab == Tab::Resources && session.settled != Tab::Resources;
        if kubed && session.mode != Mode::Diff && held {
            session.mode = Mode::Kubernetes;
        }
        if session.mode == Mode::Kubernetes && !held {
            session.mode = Mode::Agent;
        }
        session.tab = match (session.mode, session.tab) {
            (Mode::Diff, _) => Tab::Diff,
            (Mode::Files, _) => Tab::Files,
            (Mode::Kubernetes, _) => Tab::Resources,
            (_, Tab::Diff | Tab::Files | Tab::Resources) => Tab::Overview,
            (_, tab) => tab,
        };
    }

    /// The sheet shows the selected session's write only: it closes on one of another session's,
    /// and opens on arriving at a session that asks.
    fn sheeted(&mut self, app: &AppState) {
        let selected = app.session.selected.clone();
        let arrived = std::mem::replace(&mut self.session.sheeted, selected.clone()) != selected;
        let shown = self.examining().cloned();
        if shown.is_some_and(|id| crate::views::session::sheet::found(app, &id).is_none()) {
            self.close(|one| matches!(one, Overlay::Examining(_)));
        }
        let asks = selected.as_ref().and_then(|one| app.agent.activity(one));
        let first = asks.and_then(|one| one.asks.iter().find(|ask| !ask.reviewed));
        if let Some(ask) = first.filter(|_| arrived && self.overlay.is_none()) {
            self.examine(ask.id.clone());
        }
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
                session.bar.typing = None;
                session.resources.typing = false;
                if let Some(find) = session.find.as_mut() {
                    find.typing = false;
                }
                if matches!(self.overlay, Some(Overlay::Scope(_))) {
                    self.overlay = None;
                }
            }
            Surface::Session => {}
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
            (self.session.going, self.session.revealing) = (None, None);
            (self.agent.reading, self.agent.scroll) = (None, 0.0);
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
        left.typing = false;
        session.resources_of.insert(was, left);
        let back = now.and_then(|id| session.resources_of.remove(&id));
        session.resources = back.unwrap_or_default();
    }
}
