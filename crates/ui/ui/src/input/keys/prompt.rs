//! Keys at Groove's own prompt: typed into it as into a note, Enter sends it, shift+Enter breaks the line.

use groove_controllers::{AppState, Command, agent};
use groove_types::Edit;

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
    let prompt = &mut ui.agent.prompt;
    prompt.scroll = hits.typed().scroll;
    match key {
        Key::Enter if mods.shift => prompt.edit(&Edit::Newline),
        Key::Enter => return said(ui, app),
        Key::Escape if prompt.is_empty() => {
            return super::agent::to_agent(key, mods, app).into_iter().collect();
        }
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
