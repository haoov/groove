//! Keys at Groove's own prompt: typed into it as into a note, Enter sends it, shift+Enter breaks the line, Up and Down recall what was sent, Escape interrupts.

use groove_controllers::{AppState, Command, agent};
use groove_types::Edit;
use groove_types::chat::Entry;

use super::super::{Key, Modifiers};
use super::worded::asked_of_note;
use crate::Ui;
use crate::hit::Hits;
use crate::keymap::Keymap;

pub(super) fn keyed(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    (keymap, hits): (&Keymap, &Hits),
) -> Vec<Command> {
    let plain = mods == Modifiers::default();
    let prompt = &mut ui.agent.prompt;
    prompt.scroll = hits.typed().scroll;
    match key {
        Key::Up if plain && (prompt.is_empty() || prompt.recalled.is_some()) => {
            let back = prompt.recalled.unwrap_or(0) + 1;
            recalled(ui, app, back);
        }
        Key::Down if plain && prompt.recalled.is_some() => {
            let back = prompt.recalled.unwrap_or(1) - 1;
            recalled(ui, app, back);
        }
        Key::Enter if mods.shift => prompt.edit(&Edit::Newline),
        Key::Enter => return said(ui, app),
        Key::Escape if prompt.is_empty() => return interrupted(app),
        key => {
            let asked = asked_of_note(&prompt.buffer, keymap, key, mods);
            if let Some(edit) = asked.edit {
                prompt.edit(&edit);
            }
            return asked.copied.into_iter().collect();
        }
    }
    Vec::new()
}

/// The user's message `back` from the newest into the prompt; 0 empties it, past the oldest changes nothing.
fn recalled(ui: &mut Ui, app: &AppState, back: usize) {
    let Some(session) = app.session.selected.as_ref() else {
        return;
    };
    let mine = app
        .agent
        .chat(session)
        .into_iter()
        .flat_map(|one| one.entries());
    let mut mine = mine.rev().filter_map(|one| match one {
        Entry::You(text) => Some(text.as_str()),
        _ => None,
    });
    match back.checked_sub(1).map(|at| mine.nth(at)) {
        None => ui.agent.prompt.recall(None),
        Some(Some(text)) => ui.agent.prompt.recall(Some((back, text))),
        Some(None) => {}
    }
}

/// The selected session's agent stopped where it stands.
fn interrupted(app: &AppState) -> Vec<Command> {
    let session = app.session.selected.clone();
    let stop = session.map(|session| Command::Agent(agent::Command::Interrupt { session }));
    stop.into_iter().collect()
}

/// What the prompt holds, sent to the selected session's agent; the prompt empties and the conversation shows its end.
fn said(ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let text = ui.agent.prompt.text().trim().to_string();
    if text.is_empty() {
        return Vec::new();
    }
    ui.agent.prompt = Default::default();
    ui.agent.scroll = 0.0;
    vec![Command::Agent(agent::Command::Say { session, text })]
}
