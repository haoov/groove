//! The ui's own state: what stands over the surface, what the pointer holds, and the views' parts.

use groove_controllers::{AppState, Command};

use crate::{
    BoardUi, Drag, Focus, Menu, RailUi, SessionUi, Split, Surface, Target, palette, views,
};

pub use views::session::agent_pane::AgentUi;

/// What is the ui's alone: each view's part, focus, the overlay, the pointer, the caches. Never in `AppState`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Ui {
    /// Which surface the window is showing.
    pub surface: Surface,
    /// The surface shown when last settled.
    pub shown: Surface,
    pub focus: Focus,
    /// What stands over the surface and takes the next key or click.
    pub overlay: Option<Overlay>,
    pub session: SessionUi,
    pub rail: RailUi,
    pub board: BoardUi,
    pub settings: views::settings::SettingsUi,
    pub agent: AgentUi,
    pub alerts: views::alerts::AlertsUi,
    pub split: Split,
    /// The session's middle switches between modes, as the setting says.
    pub modes: bool,
    /// How far the modes layout's sidebar is scrolled, in pixels.
    pub aside: f32,
    /// What the pointer's button holds while it is down.
    pub held: Option<Held>,
    /// The last press, for the next one to know whether it carries on the same click.
    pub clicked: Option<Click>,
    /// What the pointer is over, for the row under it to say so, and where it stands.
    pub hover: Option<Target>,
    pub at: (f32, f32),
    /// The colours and the tree the last frames built, kept while they still hold.
    pub painted: views::session::diff::painted::Painted,
    pub walked: views::session::files::walked::Walked,
    pub wrapping: views::session::diff::wrapping::Wrapping,
    pub narrowing: views::session::files::Narrowing,
    pub spoken: views::session::diff::popup::Spoken,
    pub blocked: views::session::diff::blocks::Blocked,
    pub compacted: views::session::diff::wrap::Compacted,
    /// What the user moved and is still moving.
    pub motion: crate::Motion,
}

/// What stands over the surface, one at a time.
#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    Palette(palette::Palette),
    /// What the right button opened, and where.
    Menu(Menu),
    /// A change about to be thrown away, or a cluster written to, asking first.
    Losing(Losing),
    /// The write the review sheet shows.
    Examining(groove_types::ApprovalId),
    /// A scope picker's panel: what the session holds, then what it could attach.
    Scope(views::session::resources::Scoping),
    /// The agent's prompt, centred at the window's foot.
    Prompt,
}

/// What the pointer's button holds while it is down.
#[derive(Debug, Clone, PartialEq)]
pub enum Held {
    /// A boundary between columns.
    Edge(Drag),
    /// Text of the open file, being selected.
    Text,
    /// The change map's lens.
    Lens,
    /// A list's scrollbar thumb.
    Thumb(crate::hit::Scroller),
    /// A task of the plan, carried to another line.
    Task(groove_types::ExternalId),
    /// A session of the rail, pressed at `from`, `grip` below its row's top, the pointer now at `at`.
    Session {
        id: groove_types::SessionId,
        from: f32,
        grip: f32,
        at: f32,
    },
    /// A selection of our own on a terminal's grid.
    Selecting(Grid),
    /// A terminal's grid, its program sent the reports.
    Clicking(Grid),
    /// A column of the Resources list being made wider or narrower.
    Column(views::session::resources::Dragged),
    /// Text of the agent's conversation, being selected.
    Reading,
    /// Text of Groove's own prompt, being selected.
    Prompt,
}

/// A terminal's grid: one of the session's shells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grid {
    Shell(u64),
}

/// What is asked before a change is thrown away, or a cluster written to.
#[derive(Debug, Clone, PartialEq)]
pub enum Losing {
    /// One open file's tab, with the edits it owes the disk.
    Tab(String),
    /// One path of the explorer, with everything under it.
    Path(String),
    /// A write on the object of a list row.
    Row(Box<views::session::resources::Written>),
}

impl Losing {
    /// The menu row that asks it.
    pub fn act(&self) -> &'static str {
        match self {
            Losing::Row(written) => written.write.act(),
            _ => "discard",
        }
    }
}

/// A press, and how many the pointer has made in the same place in a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Click {
    pub x: f32,
    pub y: f32,
    pub at: u64,
    pub count: u32,
}

impl Ui {
    /// The surface the window shows. With no session open, the board is the window.
    pub fn showing(&self, app: &AppState) -> Surface {
        match app.session.open.is_empty() {
            true => Surface::Board,
            false => self.surface,
        }
    }

    pub fn dragging(&self) -> bool {
        matches!(self.held, Some(Held::Edge(_)))
    }

    pub fn palette(&self) -> Option<&palette::Palette> {
        match &self.overlay {
            Some(Overlay::Palette(one)) => Some(one),
            _ => None,
        }
    }

    pub fn palette_mut(&mut self) -> Option<&mut palette::Palette> {
        match &mut self.overlay {
            Some(Overlay::Palette(one)) => Some(one),
            _ => None,
        }
    }

    /// Whether the agent's prompt stands open.
    pub fn prompting(&self) -> bool {
        matches!(self.overlay, Some(Overlay::Prompt))
    }

    pub fn menu(&self) -> Option<&Menu> {
        match &self.overlay {
            Some(Overlay::Menu(one)) => Some(one),
            _ => None,
        }
    }

    pub fn losing(&self) -> Option<&Losing> {
        match &self.overlay {
            Some(Overlay::Losing(one)) => Some(one),
            _ => None,
        }
    }

    pub fn examining(&self) -> Option<&groove_types::ApprovalId> {
        match &self.overlay {
            Some(Overlay::Examining(one)) => Some(one),
            _ => None,
        }
    }

    /// The review sheet opened on one write.
    pub(crate) fn examine(&mut self, id: groove_types::ApprovalId) {
        self.overlay = Some(Overlay::Examining(id));
    }

    /// What a palette key did, applied: the palette down, Settings up; its commands returned.
    pub(crate) fn closed_palette(
        &mut self,
        outcome: palette::Outcome,
        app: &AppState,
    ) -> Vec<Command> {
        if outcome.close {
            self.overlay = None;
        }
        let mut commands = outcome.commands;
        if outcome.settings {
            commands.extend(self.open_settings());
        }
        if let Some(act) = outcome.act {
            commands.extend(crate::input::acted(act, self, app));
        }
        commands
    }

    /// Settings over the whole window, the environment and the kubeconfig read again.
    pub(crate) fn open_settings(&mut self) -> Vec<Command> {
        self.settings.open = true;
        self.overlay = None;
        vec![
            Command::Config(groove_controllers::config::Command::CheckEnvironment),
            Command::Cluster(groove_controllers::cluster::Command::ScanContexts),
        ]
    }

    /// Setup opened once, when the first check finds something left and nothing was done yet.
    pub fn settled(&mut self, app: &AppState) {
        let checked = app.config.tools.is_some() && !app.config.checking;
        if self.settings.offered || !checked {
            return;
        }
        self.settings.offered = true;
        if views::settings::rows::setup_left(app) > 0 {
            self.settings.open = true;
            self.settings.section = views::settings::Section::Setup;
            self.overlay = None;
        }
    }

    /// Back to the window; a sign-in still running ends with it.
    pub(crate) fn close_settings(&mut self) -> Vec<Command> {
        let settings = &mut self.settings;
        (settings.open, settings.typing, settings.binding) = (false, false, None);
        (settings.leaving, settings.unsharing, settings.deleting) = (None, false, None);
        vec![Command::Config(
            groove_controllers::config::Command::EndLogin,
        )]
    }

    /// The overlay taken down when `which` says it is the one standing.
    pub fn close(&mut self, which: impl Fn(&Overlay) -> bool) -> Option<Overlay> {
        match self.overlay.as_ref().is_some_and(which) {
            true => self.overlay.take(),
            false => None,
        }
    }

    /// The task a drag of the plan carries.
    pub fn carried(&self) -> Option<&groove_types::ExternalId> {
        match &self.held {
            Some(Held::Task(id)) => Some(id),
            _ => None,
        }
    }

    /// The pointer is down on something that follows it.
    pub fn pointing(&self) -> bool {
        self.held.is_some()
    }
}
