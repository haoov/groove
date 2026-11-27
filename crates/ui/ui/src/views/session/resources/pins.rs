//! The lists a session pins: the one shown as a pin, a pin brought back, the pin toggled.

use groove_controllers::session_service::Open;
use groove_controllers::{AppState, Command, session};
use groove_types::Pin;

use super::scope::{self, ResourcesUi};

/// The list as it stands, kind and scope; none while an object shows or no kind is known.
pub fn shown(app: &AppState, open: &Open, held: &ResourcesUi) -> Option<Pin> {
    if held.tab().is_some() {
        return None;
    }
    let kind = scope::kind(app, open, held)?;
    Some(Pin::new(&kind, scope::held(open, held)))
}

/// The list shown pinned, or let go when it is pinned already.
pub fn toggled(app: &AppState, held: &ResourcesUi) -> Vec<Command> {
    let Some(open) = app.session.selected() else {
        return Vec::new();
    };
    let Some(pin) = shown(app, open, held) else {
        return Vec::new();
    };
    let session = open.session.id.clone();
    let command = match open.pins.contains(&pin) {
        true => session::Command::Unpin { session, pin },
        false => session::Command::Pin { session, pin },
    };
    vec![Command::Session(command)]
}

/// The pinned list shown again: its kind, its contexts and its namespaces.
pub fn restore(held: &mut ResourcesUi, app: &AppState, open: &Open, pin: &Pin) {
    let contexts = pin.contexts();
    held.hidden_contexts = scope::contexts(open)
        .into_iter()
        .filter(|one| !contexts.contains(one))
        .map(String::from)
        .collect();
    held.picked
        .retain(|one| !contexts.contains(&one.context.as_str()));
    held.picked.extend(pin.scope.iter().cloned());
    let store = &app.cluster.store;
    let served = contexts
        .iter()
        .find_map(|one| store.served(one, &pin.kind.group, &pin.kind.kind));
    held.kind = Some(served.unwrap_or_else(|| pin.kind.clone()));
    (held.scroll, held.showing, held.typing) = (0.0, None, false);
}

/// `prod · staging / shop`: the contexts, then the namespaces, `*` for a whole cluster.
pub fn scope_of(pin: &Pin) -> String {
    let mut namespaces: Vec<&str> = pin
        .scope
        .iter()
        .map(|one| one.namespace.as_deref().unwrap_or(super::WHOLE_MARK))
        .collect();
    namespaces.sort();
    namespaces.dedup();
    format!(
        "{} / {}",
        pin.contexts().join(" · "),
        namespaces.join(" · ")
    )
}
