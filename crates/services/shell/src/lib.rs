//! The shell capability. Each session's own terminals, opened in its directory.

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use groove_types::{Error, SessionId};

pub use groove_terminal::{AnsiPalette, Hooks, PtySpec, Terminal};

/// One terminal of a session; `terminal` is `None` while it starts, or when it failed.
#[derive(Debug)]
pub struct Shell {
    pub id: u64,
    pub terminal: Option<Terminal>,
    pub failed: Option<String>,
    pub exited: Option<u32>,
}

/// One tab of the section: its terminals side by side, and the one that takes the keys.
#[derive(Debug)]
pub struct Tab {
    pub id: u64,
    pub panes: Vec<Shell>,
    pub focused: u64,
}

/// A session's tabs, in the order they stand.
#[derive(Debug, Default)]
pub struct Shells {
    pub tabs: Vec<Tab>,
    pub selected: Option<u64>,
}

impl Shells {
    pub fn get(&self, id: u64) -> Option<&Shell> {
        self.tabs
            .iter()
            .flat_map(|tab| &tab.panes)
            .find(|one| one.id == id)
    }

    pub fn tab(&self) -> Option<&Tab> {
        self.tabs.iter().find(|tab| Some(tab.id) == self.selected)
    }

    /// The selected tab's terminals, in the order they stand.
    pub fn shown(&self) -> &[Shell] {
        self.tab()
            .map(|tab| tab.panes.as_slice())
            .unwrap_or_default()
    }

    /// The terminal the keys go to.
    pub fn focused(&self) -> Option<u64> {
        self.tab().map(|tab| tab.focused)
    }

    /// Where a terminal stands among every terminal of the session, counted from one.
    pub fn number(&self, id: u64) -> usize {
        let mut all = self.tabs.iter().flat_map(|tab| &tab.panes);
        all.position(|one| one.id == id).map_or(0, |at| at + 1)
    }
}

/// The `shell` slice of `AppState`.
#[derive(Debug, Default)]
pub struct State {
    sessions: Vec<(SessionId, Shells)>,
    next: u64,
}

impl State {
    pub fn shells(&self, session: &SessionId) -> Option<&Shells> {
        let held = self.sessions.iter().find(|(id, _)| id == session);
        held.map(|(_, one)| one)
    }

    fn shells_mut(&mut self, session: &SessionId) -> &mut Shells {
        if !self.sessions.iter().any(|(id, _)| id == session) {
            self.sessions.push((session.clone(), Shells::default()));
        }
        let at = self.sessions.iter().position(|(id, _)| id == session);
        &mut self.sessions[at.unwrap_or_default()].1
    }

    fn shell_mut(&mut self, session: &SessionId, id: u64) -> Option<&mut Shell> {
        let tabs = self.shells_mut(session).tabs.iter_mut();
        tabs.flat_map(|tab| &mut tab.panes).find(|one| one.id == id)
    }

    pub fn terminal(&self, session: &SessionId, id: u64) -> Option<&Terminal> {
        self.shells(session)?.get(id)?.terminal.as_ref()
    }

    /// Every running terminal of every session in these colours.
    pub fn recolor(&self, palette: AnsiPalette) {
        let tabs = self.sessions.iter().flat_map(|(_, one)| &one.tabs);
        let panes = tabs.flat_map(|tab| &tab.panes);
        let running = panes.filter_map(|one| one.terminal.as_ref());
        running.for_each(|one| one.recolor(palette));
    }

    /// A new terminal's place while it starts: in a tab of its own, or beside the selected
    /// tab's. It takes the keys either way.
    pub fn reserve(&mut self, session: &SessionId, beside: bool) -> u64 {
        self.next += 1;
        let id = self.next;
        self.next += 1;
        let tab = self.next;
        let shell = Shell {
            id,
            terminal: None,
            failed: None,
            exited: None,
        };
        let shells = self.shells_mut(session);
        let selected = shells.selected;
        match shells.tabs.iter_mut().find(|one| Some(one.id) == selected) {
            Some(current) if beside => {
                current.panes.push(shell);
                current.focused = id;
            }
            _ => {
                shells.tabs.push(Tab {
                    id: tab,
                    panes: vec![shell],
                    focused: id,
                });
                shells.selected = Some(tab);
            }
        }
        id
    }

    /// What the spawn gave; a place closed while it started ends the terminal it got.
    pub fn started(&mut self, session: &SessionId, id: u64, result: Result<Terminal, Error>) {
        let Some(shell) = self.shell_mut(session, id) else {
            if let Ok(orphan) = result {
                let _ = orphan.terminate();
            }
            return;
        };
        match result {
            Ok(terminal) => shell.terminal = Some(terminal),
            Err(e) => shell.failed = Some(e.message),
        }
    }

    pub fn select_tab(&mut self, session: &SessionId, tab: u64) {
        let shells = self.shells_mut(session);
        if shells.tabs.iter().any(|one| one.id == tab) {
            shells.selected = Some(tab);
        }
    }

    /// The keys to one terminal, and its tab to the front.
    pub fn focus(&mut self, session: &SessionId, id: u64) {
        let shells = self.shells_mut(session);
        let held = shells
            .tabs
            .iter_mut()
            .find(|tab| tab.panes.iter().any(|one| one.id == id));
        if let Some(tab) = held {
            tab.focused = id;
            shells.selected = Some(tab.id);
        }
    }

    /// Takes one terminal out; a tab left empty goes with it. The caller ends it.
    pub fn close(&mut self, session: &SessionId, id: u64) -> Option<Shell> {
        let shells = self.shells_mut(session);
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
        let shells = self.shells_mut(session);
        let Some(at) = shells.tabs.iter().position(|one| one.id == tab) else {
            return Vec::new();
        };
        let gone = shells.tabs.remove(at).panes;
        if shells.selected == Some(tab) {
            let near = shells.tabs.get(at).or_else(|| shells.tabs.last());
            shells.selected = near.map(|one| one.id);
        }
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

/// What the reader threads tell this capability.
#[derive(Debug)]
pub enum Event {
    Damaged {
        session: SessionId,
        id: u64,
    },
    Exited {
        session: SessionId,
        id: u64,
        code: u32,
    },
}

pub fn apply(state: &mut State, event: Event) {
    match event {
        Event::Damaged { .. } => {}
        Event::Exited { session, id, code } => {
            if let Some(shell) = state.shell_mut(&session, id) {
                shell.exited = Some(code);
            }
        }
    }
}

/// The login shell `program` in `cwd`, on a grid of `cols` by `rows`; `sh` when none is named.
pub fn spec(program: &str, cwd: PathBuf, (cols, rows): (u16, u16)) -> PtySpec {
    let program = match program.is_empty() {
        true => "/bin/sh".to_string(),
        false => program.to_string(),
    };
    PtySpec {
        program,
        args: vec!["-l".to_string()],
        cwd,
        env: vec![("TERM".to_string(), "xterm-256color".to_string())],
        rows,
        cols,
    }
}

pub fn spawn(spec: PtySpec, palette: AnsiPalette, hooks: Hooks) -> Result<Terminal, Error> {
    Terminal::spawn(spec, palette, hooks)
        .map_err(|e| Error::new(groove_types::ErrorKind::Io, e.to_string()))
}
