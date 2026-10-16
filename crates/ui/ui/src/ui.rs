//! The ui's own state: what stands over the surface, what the pointer holds, and the views' parts.

use groove_controllers::{AppState, Command};

use crate::{
    BoardUi, Drag, Focus, Menu, RailUi, SessionUi, Split, Surface, Target, palette, views,
};

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
    pub split: Split,
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
}

/// What stands over the surface, one at a time.
#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    Palette(palette::Palette),
    /// What the right button opened, and where.
    Menu(Menu),
    /// A change about to be thrown away, asking first.
    Losing(Losing),
    /// The write the review sheet shows.
    Examining(groove_types::ApprovalId),
    /// A scope picker's panel: what the session holds, then what it could attach.
    Scope(views::session::resources::Scoping),
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
    /// A task of the plan, carried to another line.
    Task(groove_types::ExternalId),
    /// A selection of our own on a terminal's grid.
    Selecting(Grid),
    /// A terminal's grid, its program sent the reports.
    Clicking(Grid),
    /// A column of the Resources list being made wider or narrower.
    Column(views::session::resources::Dragged),
}

/// A terminal's grid: the agent's, or one of the session's shells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grid {
    Agent,
    Shell(u64),
}

/// What the pointer is doing to the agent's screen.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct AgentUi {
    /// Wheel pixels not yet worth a line.
    pub carried: f32,
    /// Shift was held: the selection is ours even where the program reads the mouse.
    pub bypassed: bool,
}

/// What is asked before a change is thrown away.
#[derive(Debug, Clone, PartialEq)]
pub enum Losing {
    File(String),
    /// One open file's tab, with the edits it owes the disk.
    Tab(String),
    /// One path of the explorer, with everything under it.
    Path(String),
    Everything,
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
        if let (true, Some(open)) = (outcome.scoping, app.session.selected()) {
            self.surface = Surface::Session;
            let scoping = views::session::resources::Scoping::asked(open);
            let (scoping, listing) = scoping.opened(open, &self.session.resources);
            self.overlay = Some(Overlay::Scope(scoping));
            commands.extend(listing);
        }
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
