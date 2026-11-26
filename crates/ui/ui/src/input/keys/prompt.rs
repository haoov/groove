//! Keys for the agent: at rest they answer what it asks, Escape interrupts it; in the open prompt
//! they type as into a note, Enter sends and closes, Escape closes and keeps the draft.

use groove_controllers::{AppState, Command, agent};
use groove_types::Edit;
use groove_types::chat::Entry;

use super::super::{Key, Modifiers, answer};
use super::worded::asked_of_note;
use crate::hit::Hits;
use crate::keymap::Keymap;
use crate::views::session::agent_pane::offered::offered;
use crate::{Overlay, Ui};

/// The prompt opened with the keys, given them back when they went elsewhere, or closed.
pub(super) fn toggled(ui: &mut Ui, app: &AppState) {
    let had = ui.focus == crate::Focus::Agent;
    ui.focus = crate::Focus::Agent;
    if ui.prompting() && !had {
        return;
    }
    if closed(ui) {
        return;
    }
    if ui.showing(app) == crate::Surface::Session && app.session.selected.is_some() {
        ui.overlay = Some(Overlay::Prompt);
    }
}

/// The prompt closed, its draft kept; whether it stood open.
pub(super) fn closed(ui: &mut Ui) -> bool {
    ui.close(|one| matches!(one, Overlay::Prompt)).is_some()
}

/// A key while the pane has the keys and the prompt is shut.
pub(super) fn keyed((key, mods): (Key, Modifiers), ui: &mut Ui, app: &AppState) -> Vec<Command> {
    if mods != Modifiers::default() {
        return Vec::new();
    }
    if let Some(button) = answer::keyed(app, ui, key) {
        return answer::answered(ui, app, button);
    }
    match key {
        Key::Escape => interrupted(app),
        _ => Vec::new(),
    }
}

/// A key in the open prompt.
pub(super) fn typed(
    (key, mods): (Key, Modifiers),
    ui: &mut Ui,
    app: &AppState,
    (keymap, hits): (&Keymap, &Hits),
) -> Vec<Command> {
    let plain = mods == Modifiers::default();
    if let Some(took) = offering(key, plain, ui, app) {
        return took;
    }
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
        Key::Escape => _ = closed(ui),
        key => {
            let asked = asked_of_note(&prompt.buffer, keymap, key, mods);
            if let Some(edit) = asked.edit {
                prompt.edit(&edit);
                (ui.agent.offered, ui.agent.hushed) = (0, false);
            }
            return asked.copied.into_iter().collect();
        }
    }
    Vec::new()
}

/// A key on the commands `/` offers: Up and Down choose, Tab or Enter takes one, Escape puts them away.
fn offering(key: Key, plain: bool, ui: &mut Ui, app: &AppState) -> Option<Vec<Command>> {
    let offers = offered(app, ui);
    if offers.is_empty() || !plain {
        return None;
    }
    let chosen = ui.agent.offered.min(offers.len() - 1);
    let name = offers[chosen].label.to_string();
    let whole = ui.agent.prompt.text() == format!("/{name}");
    match key {
        Key::Up => ui.agent.offered = chosen.checked_sub(1).unwrap_or(offers.len() - 1),
        Key::Down => ui.agent.offered = (chosen + 1) % offers.len(),
        Key::Escape => ui.agent.hushed = true,
        Key::Tab => crate::input::ask::command(ui, &name),
        Key::Enter if !whole => crate::input::ask::command(ui, &name),
        _ => return None,
    }
    Some(Vec::new())
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

/// What the prompt holds, sent to the selected session's agent; the prompt empties and closes, and the conversation shows its end.
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
    closed(ui);
    vec![Command::Agent(agent::Command::Say { session, text })]
}
