//! The surfaces: `view` draws the state into a `Frame`, `input` turns keys and clicks into commands.

use groove_controllers::{AppState, Command};

mod components;
mod ctx;
mod hit;
pub mod input;
pub mod keymap;
mod layout;
mod menu;
mod offsets;
pub mod palette;
mod render;
mod views;

#[cfg(test)]
mod tests;

pub use groove_ui_kit::base::ctx::Metrics;
pub use groove_ui_kit::base::mark::Mark;
pub use groove_ui_kit::base::style::Role;
pub use groove_ui_kit::base::tokens::{SPINNER_MS, Tokens};
pub use groove_ui_kit::widgets::Corner;
pub use hit::{Cursor, Hits, Target};
pub use layout::{Edge, Split};
pub use menu::{Menu, Of, Offer};
pub use render::{frame_commands, view};
pub use views::board::BoardUi;
pub use views::rail::RailUi;
pub use views::session::{Asked, Naming, SessionUi, Tab};

/// What the window shows beside the rail.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    #[default]
    Session,
    Board,
}

/// Which pane the keyboard belongs to.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Rail,
    #[default]
    Agent,
    Workspace,
    /// The manual section's selected terminal.
    Terminal,
    Sidebar,
}

impl Focus {
    /// The panes left to right, for a chord that moves between them.
    pub const ALL: [Focus; 4] = [Focus::Rail, Focus::Agent, Focus::Workspace, Focus::Sidebar];

    /// The pane beside this one, or this one at the edge.
    pub fn beside(self, right: bool) -> Self {
        let from = match self {
            Focus::Terminal => Focus::Workspace,
            one => one,
        };
        let at = Self::ALL.iter().position(|it| *it == from).unwrap_or(1);
        let next = match right {
            true => at + 1,
            false => at.saturating_sub(1),
        };
        Self::ALL.get(next).copied().unwrap_or(from)
    }
}

/// A boundary under the pointer: which one, and where the pointer took hold of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drag {
    pub edge: Edge,
    /// The pointer's distance from the boundary when it was grabbed, in logical pixels.
    pub offset: f32,
}

/// What is the ui's alone: focus, folds, the palette, the splits. Never in `AppState`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Ui {
    /// Which surface the window is showing.
    pub surface: Surface,
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
    /// A selection of the agent's screen, of our own.
    AgentText,
    /// The agent's screen, its program sent the reports.
    AgentClick,
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

    /// What the layout reads from the state: a routine's session selected, a commit shown.
    pub fn settle(&mut self, app: &AppState) {
        let selected = app.session.selected().map(|open| &open.session.kind);
        self.session.alone = selected.is_some_and(|kind| kind.routine().is_some());
        self.session.reading = app.workspace.commit.is_some();
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
    pub(crate) fn closed_palette(&mut self, outcome: palette::Outcome) -> Vec<Command> {
        if outcome.close {
            self.overlay = None;
        }
        let mut commands = outcome.commands;
        if outcome.settings {
            commands.extend(self.open_settings());
        }
        commands
    }

    /// Settings over the whole window, the environment checked again.
    pub(crate) fn open_settings(&mut self) -> Vec<Command> {
        self.settings.open = true;
        self.overlay = None;
        vec![Command::Config(
            groove_controllers::config::Command::CheckEnvironment,
        )]
    }

    /// Back to the window; a sign-in still running ends with it.
    pub(crate) fn close_settings(&mut self) -> Vec<Command> {
        self.settings.open = false;
        self.settings.typing = false;
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
