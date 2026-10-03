//! What the filter offers for the token being typed.

use groove_controllers::AppState;

use super::filter::Name;

/// How many suggestions the field shows at once.
pub const ROWS: usize = 6;

/// The whole filter text, with its last token replaced by `pick`.
pub fn taken(text: &str, pick: &str) -> String {
    let kept = text
        .rsplit_once(char::is_whitespace)
        .map(|(kept, _)| kept)
        .unwrap_or_default();
    match kept.is_empty() {
        true => pick.to_string(),
        false => format!("{kept} {pick}"),
    }
}

/// What the token being typed could become: a field to name, or a value it holds.
pub fn offers(app: &AppState, text: &str) -> Vec<String> {
    let token = text.split_whitespace().next_back().unwrap_or_default();
    if text.ends_with(char::is_whitespace) || token.is_empty() {
        return fields("");
    }
    let offers = match token.split_once(':') {
        Some((name, typed)) => match Name::ALL.into_iter().find(|one| one.as_str() == name) {
            Some(name) => values(app, name, typed),
            None => Vec::new(),
        },
        None => fields(token),
    };
    unless_whole(offers, token)
}

/// A token that already reads as the only thing it could become is offered nothing.
fn unless_whole(offers: Vec<String>, token: &str) -> Vec<String> {
    match offers.iter().any(|one| one.eq_ignore_ascii_case(token)) {
        true => Vec::new(),
        false => offers,
    }
}

/// Every field the filter knows, as the token that names it.
fn fields(typed: &str) -> Vec<String> {
    Name::ALL
        .into_iter()
        .map(|name| format!("{}:", name.as_str()))
        .filter(|one| one.starts_with(typed))
        .collect()
}

/// The values the board itself holds for that field.
fn values(app: &AppState, name: Name, typed: &str) -> Vec<String> {
    let held: Vec<String> = match name {
        Name::Status => app.task.tasks.iter().map(|t| t.status.clone()).collect(),
        Name::Priority => groove_types::Priority::ALL
            .iter()
            .map(|one| one.label().to_string())
            .collect(),
        Name::Project => app
            .task
            .tasks
            .iter()
            .filter_map(|t| t.project.clone())
            .collect(),
        Name::Provider => groove_types::ProviderId::ALL
            .iter()
            .map(|one| one.as_str().to_string())
            .collect(),
        Name::Kind => groove_types::SessionKind::NAMES
            .iter()
            .map(|one| (*one).to_string())
            .collect(),
        Name::Repo => app
            .session
            .living
            .iter()
            .flat_map(|living| living.worktrees.iter())
            .map(|worktree| worktree.repo.as_str().to_string())
            .collect(),
    };
    let wanted = typed.to_lowercase();
    let mut offers: Vec<String> = held
        .into_iter()
        .filter(|one| !one.is_empty() && one.to_lowercase().contains(&wanted))
        .map(|one| format!("{}:{}", name.as_str(), one.replace(' ', "-")))
        .collect();
    offers.sort();
    offers.dedup();
    offers
}
