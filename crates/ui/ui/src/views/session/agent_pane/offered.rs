//! The commands the prompt offers while its one word starts with `/`: the agent's own, then
//! Groove's skills.

use groove_controllers::AppState;
use groove_ui_kit::widgets::Offer;

use crate::Ui;

/// The commands that hold what follows `/`, the agent's first; none once hushed.
pub(crate) fn offered<'a>(app: &'a AppState, ui: &Ui) -> Vec<Offer<'a>> {
    grouped(app, ui).0
}

/// The commands offered, the agent's then Groove's, and how many are the agent's.
pub(crate) fn grouped<'a>(app: &'a AppState, ui: &Ui) -> (Vec<Offer<'a>>, usize) {
    let text = ui.agent.prompt.text();
    let Some(asked) = text
        .strip_prefix('/')
        .filter(|one| !one.contains(char::is_whitespace))
    else {
        return (Vec::new(), 0);
    };
    if ui.agent.hushed {
        return (Vec::new(), 0);
    }
    let asked = asked.to_lowercase();
    let found = |list: Vec<Offer<'a>>| {
        let mut found: Vec<Offer<'a>> = list
            .into_iter()
            .filter(|one| one.label.to_lowercase().contains(&asked))
            .collect();
        found.sort_by_key(|one| !one.label.to_lowercase().starts_with(&asked));
        found
    };
    let (own, groove) = listed(app);
    let own = found(own);
    let count = own.len();
    (own.into_iter().chain(found(groove)).collect(), count)
}

/// The agent's own commands, and Groove's skills: those the agent lists, else Groove's own list.
fn listed<'a>(app: &'a AppState) -> (Vec<Offer<'a>>, Vec<Offer<'a>>) {
    let session = app.session.selected.as_ref();
    let agent = session.and_then(|one| app.agent.agent(one));
    let own = agent.map(|one| one.commands.as_slice()).unwrap_or_default();
    let skills = app.skill.skills.iter();
    if own.is_empty() {
        let skills = skills.map(|one| Offer {
            label: &one.id,
            detail: &one.hint,
        });
        return (Vec::new(), skills.collect());
    }
    let groove = |name: &str| app.skill.skills.iter().any(|one| one.id == name);
    let offer = |one: &'a groove_types::chat::Slash| Offer {
        label: &one.name,
        detail: &one.about,
    };
    let (theirs, ours): (Vec<_>, Vec<_>) = own.iter().partition(|one| !groove(&one.name));
    (
        theirs.into_iter().map(offer).collect(),
        ours.into_iter().map(offer).collect(),
    )
}
