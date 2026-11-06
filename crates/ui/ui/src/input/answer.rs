//! The answer to a headless agent's first ask: leave, a refusal, or its questions' options picked one by one.

use groove_controllers::{AppState, Command, agent};

use crate::Ui;
use crate::hit::Answering;

/// The answer sent once it is whole; a pick short of the last question is only held.
pub(crate) fn answered(ui: &mut Ui, app: &AppState, button: Answering) -> Vec<Command> {
    let Some(session) = app.session.selected.clone() else {
        return Vec::new();
    };
    let Some(asked) = app.agent.permissions(&session).first() else {
        return Vec::new();
    };
    let mut picked = ui.agent.picked(&asked.id);
    let allow = match button {
        Answering::Refuse => false,
        Answering::Allow if asked.questions.is_empty() => true,
        Answering::Allow => return Vec::new(),
        Answering::Pick(option) => {
            picked.push(option);
            if picked.len() < asked.questions.len() {
                ui.agent.picked = Some((asked.id.clone(), picked));
                return Vec::new();
            }
            true
        }
    };
    ui.agent.picked = None;
    let id = asked.id.clone();
    vec![Command::Agent(agent::Command::Answer {
        session,
        id,
        allow,
        picked,
    })]
}

/// What a key answers while the prompt is empty and an ask waits: Enter leaves, Escape refuses, a digit picks.
pub(crate) fn keyed(app: &AppState, ui: &Ui, key: crate::input::Key) -> Option<Answering> {
    let session = app.session.selected.as_ref()?;
    let asked = app.agent.permissions(session).first()?;
    match key {
        crate::input::Key::Enter => Some(Answering::Allow),
        crate::input::Key::Escape => Some(Answering::Refuse),
        crate::input::Key::Char(digit) => {
            let question = asked.questions.get(ui.agent.picked(&asked.id).len())?;
            let at = digit.to_digit(10)?.checked_sub(1)?;
            question
                .options
                .get(at as usize)
                .cloned()
                .map(Answering::Pick)
        }
        _ => None,
    }
}
