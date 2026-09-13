//! The command palette: every action of every capability, filtered by what you type.

use groove_controllers::{AppState, Command, session};

use crate::input::Key;

/// One row of the palette. Its id is the command's, so a row is always a real function.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub group: &'static str,
    pub label: String,
    pub command: Command,
}

impl Entry {
    pub fn id(&self) -> &'static str {
        self.command.id()
    }
}

/// The palette's own state while it is open.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Palette {
    pub query: String,
    pub selected: usize,
}

/// Every entry the state allows right now, in group order.
pub fn entries(app: &AppState) -> Vec<Entry> {
    let mut out = vec![Entry {
        group: "Session",
        label: "New explorer".into(),
        command: Command::Session(session::Command::OpenExplorer { title: None }),
    }];
    if let Some(selected) = &app.session.selected {
        out.push(Entry {
            group: "Session",
            label: "Close session".into(),
            command: Command::Session(session::Command::Close {
                session: selected.clone(),
            }),
        });
    }
    for open in &app.session.open {
        if app.session.selected.as_ref() == Some(&open.session.id) {
            continue;
        }
        out.push(Entry {
            group: "Session",
            label: format!("Switch to {}", open.session.title),
            command: Command::Session(session::Command::Select {
                session: open.session.id.clone(),
            }),
        });
    }
    out
}

/// The entries whose group or label holds every word of the query, best match first.
pub fn matching(entries: Vec<Entry>, query: &str) -> Vec<Entry> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut scored: Vec<(usize, Entry)> = entries
        .into_iter()
        .filter_map(|e| {
            let hay = format!("{} {}", e.group, e.label).to_lowercase();
            let score = words
                .iter()
                .map(|w| hay.find(w.as_str()))
                .sum::<Option<usize>>()?;
            Some((score, e))
        })
        .collect();
    scored.sort_by_key(|(score, _)| *score);
    scored.into_iter().map(|(_, e)| e).collect()
}

impl Palette {
    /// A key while the palette is open. `Some` is the command to run; the palette closes.
    pub fn key(&mut self, key: Key, app: &AppState) -> Option<Command> {
        let rows = self.rows(app);
        match key {
            Key::Char(c) => {
                self.query.push(c);
                self.selected = 0;
            }
            Key::Backspace => {
                self.query.pop();
                self.selected = 0;
            }
            Key::Down => self.selected = (self.selected + 1).min(rows.len().saturating_sub(1)),
            Key::Up => self.selected = self.selected.saturating_sub(1),
            Key::Enter => return rows.into_iter().nth(self.selected).map(|e| e.command),
            _ => {}
        }
        None
    }

    pub fn rows(&self, app: &AppState) -> Vec<Entry> {
        matching(entries(app), &self.query)
    }
}
