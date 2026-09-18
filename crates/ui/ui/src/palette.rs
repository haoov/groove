//! The command palette: every action of every capability, filtered by what you type.
//! An action that needs arguments asks for them one prompt at a time.

mod flow;

use groove_controllers::{AppState, Command, session};

pub use flow::{Action, CLONE, Flow, Prompt};

use crate::Corner;
use crate::input::Key;

/// What picking a row does: run a command, or start asking for arguments.
#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    Command(Command),
    Flow(Action),
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

/// What a key did: commands to dispatch now, and whether the palette closes.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    pub commands: Vec<Command>,
    pub close: bool,
}

/// The palette's own state while it is open.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Palette {
    pub query: String,
    pub selected: usize,
    pub flow: Option<Flow>,
    /// Where it is drawn: on what opened it, or in the middle of the window.
    pub anchor: Option<Anchor>,
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

/// Every entry the state allows right now, in group order.
pub fn entries(app: &AppState) -> Vec<Entry> {
    let mut out = vec![Entry::command(
        "Session",
        "New explorer",
        Command::Session(session::Command::OpenExplorer { title: None }),
    )];
    let Some(open) = app.session.selected() else {
        return out;
    };
    let id = open.session.id.clone();
    out.push(Entry::flow("Session", "Add repo", Action::AddRepo));
    if !open.repos.is_empty() {
        out.push(Entry::flow("Session", "Add worktree", Action::AddWorktree));
        out.push(Entry::flow("Session", "Remove repo", Action::RemoveRepo));
    }
    if !open.worktrees.is_empty() {
        out.push(Entry::flow(
            "Session",
            "Close worktree",
            Action::CloseWorktree,
        ));
    }
    if open.worktrees.len() > 1 {
        out.push(Entry::flow(
            "Session",
            "Select worktree",
            Action::SelectWorktree,
        ));
    }
    if matches!(open.session.kind, groove_types::SessionKind::Explorer) {
        out.push(Entry::flow(
            "Session",
            "Rename explorer",
            Action::RenameExplorer,
        ));
        out.push(Entry::command(
            "Session",
            "Delete session",
            Command::Session(session::Command::Delete {
                session: id.clone(),
            }),
        ));
    }
    out.push(Entry::command(
        "Session",
        "Close session",
        Command::Session(session::Command::Close {
            session: id.clone(),
        }),
    ));
    for other in app.session.open.iter().filter(|o| o.session.id != id) {
        out.push(Entry::command(
            "Session",
            format!("Switch to {}", other.session.title),
            Command::Session(session::Command::Select {
                session: other.session.id.clone(),
            }),
        ));
    }
    out
}

/// The rows whose text holds every word of the query, best match first.
pub fn matching<T>(rows: Vec<T>, text: impl Fn(&T) -> String, query: &str) -> Vec<T> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut scored: Vec<(usize, T)> = rows
        .into_iter()
        .filter_map(|row| {
            let hay = text(&row).to_lowercase();
            let score = words
                .iter()
                .map(|w| hay.find(w.as_str()))
                .sum::<Option<usize>>()?;
            Some((score, row))
        })
        .collect();
    scored.sort_by_key(|(score, _)| *score);
    scored.into_iter().map(|(_, row)| row).collect()
}

impl Palette {
    /// The rows under the query: entries, or the current prompt's options.
    pub fn rows(&self, app: &AppState) -> Vec<Entry> {
        matching(
            entries(app),
            |e| format!("{} {}", e.group, e.label),
            &self.query,
        )
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
        let mut rows = matching(rest, |(label, _)| label.clone(), &self.query);
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
        match key {
            Key::Char(c) => {
                self.query.push(c);
                self.selected = 0;
            }
            Key::Backspace => {
                self.query.pop();
                self.selected = 0;
            }
            Key::Down => {
                self.selected = (self.selected + 1).min(self.row_count(app).saturating_sub(1))
            }
            Key::Up => self.selected = self.selected.saturating_sub(1),
            Key::Enter => return self.enter(app),
            Key::Escape => return self.escape(),
            _ => {}
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
            },
            Run::Flow(action) => {
                let Some(session) = app.session.selected.clone() else {
                    return Outcome::default();
                };
                let flow = Flow::new(action, session);
                let refresh = flow.refresh(app);
                self.flow = Some(flow);
                self.reset();
                Outcome {
                    commands: refresh.into_iter().collect(),
                    close: false,
                }
            }
        }
    }

    /// The typed text or the picked option becomes the answer; the flow moves on or ends.
    fn answer(&mut self, app: &AppState) -> Outcome {
        let Some(prompt) = self.prompt(app) else {
            return Outcome::default();
        };
        let picked = self
            .options(app)
            .into_iter()
            .nth(self.selected)
            .map(|(_, value)| value);
        let value = match picked {
            Some(value)
                if !prompt.options.is_empty() && !(self.query.is_empty() && prompt.allow_empty) =>
            {
                value
            }
            _ if prompt.free || (prompt.allow_empty && self.query.is_empty()) => self.query.clone(),
            _ => return Outcome::default(),
        };
        if value.is_empty() && !prompt.allow_empty {
            return Outcome::default();
        }
        let Some(flow) = self.flow.as_mut() else {
            return Outcome::default();
        };
        flow.answers.push(value);
        self.reset();
        let Some(flow) = self.flow.as_ref() else {
            return Outcome::default();
        };
        if flow.prompt(app).is_some() {
            return Outcome {
                commands: flow.refresh(app).into_iter().collect(),
                close: false,
            };
        }
        let command = flow.command();
        self.flow = None;
        Outcome {
            commands: command.into_iter().collect(),
            close: true,
        }
    }

    /// Back one prompt; out of the flow; closed.
    fn escape(&mut self) -> Outcome {
        match self.flow.as_mut() {
            Some(flow) if !flow.answers.is_empty() => {
                flow.answers.pop();
                self.reset();
                Outcome::default()
            }
            Some(_) => {
                self.flow = None;
                self.reset();
                Outcome::default()
            }
            None => Outcome {
                commands: Vec::new(),
                close: true,
            },
        }
    }

    fn reset(&mut self) {
        self.query.clear();
        self.selected = 0;
    }
}
