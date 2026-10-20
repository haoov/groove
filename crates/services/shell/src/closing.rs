//! A session's terminals taken out: one, a tab of them, or every one; the caller ends them.

use groove_types::SessionId;

use crate::{Shell, State};

impl State {
    /// Takes one terminal out; a tab left empty goes with it. The caller ends it.
    pub fn close(&mut self, session: &SessionId, id: u64) -> Option<Shell> {
        let gone = self.closed(session, id);
        self.emptied();
        gone
    }

    fn closed(&mut self, session: &SessionId, id: u64) -> Option<Shell> {
        let shells = self.held_mut(session)?;
        let at = shells
            .tabs
            .iter()
            .position(|tab| tab.panes.iter().any(|one| one.id == id))?;
        let tab = &mut shells.tabs[at];
        let pane = tab.panes.iter().position(|one| one.id == id)?;
        let gone = tab.panes.remove(pane);
        if let Some(near) = tab.panes.get(pane).or_else(|| tab.panes.last()) {
            if tab.focused == id {
                tab.focused = near.id;
            }
            return Some(gone);
        }
        let tab = shells.tabs.remove(at).id;
        if shells.selected == Some(tab) {
            let near = shells.tabs.get(at).or_else(|| shells.tabs.last());
            shells.selected = near.map(|one| one.id);
        }
        Some(gone)
    }

    /// One tab taken out with every terminal in it, for the caller to end.
    pub fn close_tab(&mut self, session: &SessionId, tab: u64) -> Vec<Shell> {
        let Some(shells) = self.held_mut(session) else {
            return Vec::new();
        };
        let Some(at) = shells.tabs.iter().position(|one| one.id == tab) else {
            return Vec::new();
        };
        let gone = shells.tabs.remove(at).panes;
        if shells.selected == Some(tab) {
            let near = shells.tabs.get(at).or_else(|| shells.tabs.last());
            shells.selected = near.map(|one| one.id);
        }
        self.emptied();
        gone
    }

    /// Every terminal of a session taken out, for the caller to end.
    pub fn end(&mut self, session: &SessionId) -> Vec<Shell> {
        let Some(at) = self.sessions.iter().position(|(id, _)| id == session) else {
            return Vec::new();
        };
        let tabs = self.sessions.remove(at).1.tabs;
        tabs.into_iter().flat_map(|tab| tab.panes).collect()
    }
}
