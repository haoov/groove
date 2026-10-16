//! The command palette: every action, filtered by what is typed, arguments asked one at a time.

mod actions;
mod entries;
mod files;
mod flow;
mod grouped;
mod marks;
mod mr;
mod skills;
mod steps;

use groove_controllers::{AppState, Command};

pub use actions::Act;
pub use entries::entries;
pub use flow::{Action, CLONE, Flow, Prompt};
pub use marks::mark_of;

use crate::Corner;
use groove_ui_kit::widgets::Field;

use crate::input::Key;

/// What picking a row does: run a command, or start asking for arguments.
#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    Command(Command),
    Flow(Action),
    /// The Settings surface, which is the window's own and asks no command.
    Settings,
    /// The namespace picker's panel, under its picker.
    Scope,
    Act(Act),
}

/// One row of the palette. A command row's id is the command's own.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub group: &'static str,
    pub label: String,
    pub run: Run,
}

impl Entry {
    pub fn id(&self) -> &'static str {
        match &self.run {
            Run::Command(c) => c.id(),
            Run::Flow(a) => a.id(),
            Run::Settings => "settings.open",
            Run::Scope => "resources.select_namespace",
            Run::Act(act) => act.id(),
        }
    }

    fn command(group: &'static str, label: impl Into<String>, command: Command) -> Self {
        Self {
            group,
            label: label.into(),
            run: Run::Command(command),
        }
    }

    fn flow(group: &'static str, label: impl Into<String>, action: Action) -> Self {
        Self {
            group,
            label: label.into(),
            run: Run::Flow(action),
        }
    }
}

/// What a key did: commands to dispatch now, whether the palette closes, and whether Settings opens.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    pub commands: Vec<Command>,
    pub close: bool,
    pub settings: bool,
    /// The namespace picker's panel opens.
    pub scoping: bool,
    pub act: Option<Act>,
}

/// The palette's own state while it is open.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Palette {
    pub query: Field,
    pub selected: usize,
    pub flow: Option<Flow>,
    /// Where it is drawn: on what opened it, or in the middle of the window.
    pub anchor: Option<Anchor>,
    /// A key has moved or narrowed the rows since it opened.
    pub keyed: bool,
    pub(crate) found: files::Found,
}

/// The corner of the panel, and where that corner sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub at: (u32, u32),
    pub corner: Corner,
}

impl Anchor {
    /// Under the rect that opened it, along its left edge.
    pub fn under(rect: groove_gfx::Rect) -> Self {
        Self {
            at: (rect.x as u32, rect.bottom() as u32),
            corner: Corner::TopLeft,
        }
    }

    pub(crate) fn point(&self) -> (f32, f32) {
        (self.at.0 as f32, self.at.1 as f32)
    }
}

/// The rows the query finds, the best fuzzy match first.
pub fn matching<T>(rows: Vec<T>, text: impl Fn(&T) -> String, query: &str) -> Vec<T> {
    groove_types::ranked(rows, text, query)
}

impl Palette {
    /// The rows under the query: entries, or the current prompt's options.
    pub fn rows(&self, app: &AppState) -> Vec<Entry> {
        match self.query.text().strip_prefix('#') {
            Some(rest) => files::rows(app, rest, &self.found),
            None => grouped::grouped(entries(app), self.query.text()),
        }
    }

    /// The prompt a flow is on, if any.
    pub fn prompt(&self, app: &AppState) -> Option<Prompt> {
        self.flow.as_ref().and_then(|f| f.prompt(app))
    }

    /// The prompt's options under the query; the clone row is always there.
    pub fn options(&self, app: &AppState) -> Vec<(String, String)> {
        let Some(prompt) = self.prompt(app) else {
            return Vec::new();
        };
        let (fixed, rest): (Vec<_>, Vec<_>) =
            prompt.options.into_iter().partition(|(_, v)| v == CLONE);
        let mut rows = matching(rest, |(label, _)| label.clone(), self.query.text());
        rows.extend(fixed);
        rows
    }

    /// The number of rows the selection moves over.
    fn row_count(&self, app: &AppState) -> usize {
        match &self.flow {
            Some(_) => self.options(app).len(),
            None => self.rows(app).len(),
        }
    }

    pub fn key(&mut self, key: Key, app: &AppState) -> Outcome {
        let typed = matches!(key, Key::Char(_) | Key::Backspace | Key::Delete);
        if typed || matches!(key, Key::Down | Key::Up) {
            self.keyed = true;
        }
        if typed {
            self.selected = 0;
        }
        match key {
            Key::Char(c) => self.query.insert(c),
            Key::Backspace if self.query.is_empty() => self.back(),
            Key::Backspace => self.query.backspace(),
            Key::Delete => self.query.delete(),
            Key::Left => self.query.left(),
            Key::Right => self.query.right(),
            Key::Home => self.query.home(),
            Key::End => self.query.end(),
            Key::Down => {
                self.selected = (self.selected + 1).min(self.row_count(app).saturating_sub(1))
            }
            Key::Up => self.selected = self.selected.saturating_sub(1),
            Key::Enter => return self.enter(app),
            Key::Escape => return self.escape(),
            _ => {}
        }
        if typed && self.flow.is_none() && self.query.text() == "#" {
            let list = groove_controllers::workspace::Command::ListPaths;
            return Outcome {
                commands: vec![Command::Workspace(list)],
                ..Outcome::default()
            };
        }
        Outcome::default()
    }

    fn enter(&mut self, app: &AppState) -> Outcome {
        if self.flow.is_some() {
            return self.answer(app);
        }
        let Some(entry) = self.rows(app).into_iter().nth(self.selected) else {
            return Outcome::default();
        };
        match entry.run {
            Run::Command(command) => Outcome {
                commands: vec![command],
                close: true,
                ..Outcome::default()
            },
            Run::Scope => Outcome {
                close: true,
                scoping: true,
                ..Outcome::default()
            },
            Run::Settings => Outcome {
                close: true,
                settings: true,
                ..Outcome::default()
            },
            Run::Act(act) => Outcome {
                close: true,
                act: Some(act),
                ..Outcome::default()
            },
            Run::Flow(action) => {
                let session = match (app.session.selected.clone(), action) {
                    (Some(session), _) => session,
                    (None, Action::ForceDelete) => groove_types::SessionId::new(""),
                    (None, _) => return Outcome::default(),
                };
                let mut flow = Flow::new(action, session);
                flow.from = (entry.label, self.query.text().to_string());
                let refresh = flow.refresh(app);
                self.flow = Some(flow);
                self.reset();
                Outcome {
                    commands: refresh.into_iter().collect(),
                    ..Outcome::default()
                }
            }
        }
    }

    fn reset(&mut self) {
        self.query.clear();
        self.selected = 0;
        self.keyed = false;
    }
}
