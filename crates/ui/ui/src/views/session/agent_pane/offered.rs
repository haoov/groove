//! The commands the prompt offers while its one word starts with `/`: the agent's own when it
//! runs headless, else Groove's skills.

use groove_controllers::AppState;
use groove_ui_kit::widgets::Offer;

use crate::Ui;

/// The commands that hold what follows `/`, those that start with it first; none once hushed.
pub(crate) fn offered<'a>(app: &'a AppState, ui: &Ui) -> Vec<Offer<'a>> {
    let text = ui.agent.prompt.text();
    let Some(asked) = text
        .strip_prefix('/')
        .filter(|one| !one.contains(char::is_whitespace))
    else {
        return Vec::new();
    };
    if ui.agent.hushed {
        return Vec::new();
    }
    let asked = asked.to_lowercase();
    let mut found: Vec<Offer> = listed(app)
        .into_iter()
        .filter(|one| one.label.to_lowercase().contains(&asked))
        .collect();
    found.sort_by_key(|one| !one.label.to_lowercase().starts_with(&asked));
    found
}

fn listed<'a>(app: &'a AppState) -> Vec<Offer<'a>> {
    let session = app.session.selected.as_ref();
    let agent = session.and_then(|one| app.agent.agent(one));
    let own = agent.map(|one| one.commands.as_slice()).unwrap_or_default();
    if !own.is_empty() {
        let offer = |one: &'a groove_types::chat::Slash| Offer {
            label: &one.name,
            detail: &one.about,
        };
        return own.iter().map(offer).collect();
    }
    let skills = app.skill.skills.iter();
    skills
        .map(|one| Offer {
            label: &one.id,
            detail: &one.hint,
        })
        .collect()
}
