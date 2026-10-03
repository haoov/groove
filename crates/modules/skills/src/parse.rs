//! What a `SKILL.md` says about itself: its front matter, one line a field.

use groove_types::front_matter::{field, listed, split};
use groove_types::{SessionKind, Skill, Timestamp};

/// One skill as its file describes it; its directory is its identity, never `name:`.
pub(crate) fn skill(
    plugin: &str,
    name: &str,
    body: &str,
    editable: bool,
    changed_at: Timestamp,
) -> Skill {
    let front = split(body).map_or("", |(front, _)| front);
    Skill {
        id: format!("{plugin}:{name}"),
        plugin: plugin.to_string(),
        name: name.to_string(),
        description: field(front, "description").unwrap_or_default(),
        hint: field(front, "groove-hint").unwrap_or_default(),
        label: field(front, "groove-label").unwrap_or_else(|| name.replace('-', " ")),
        kinds: kinds(field(front, "groove-kinds").as_deref()),
        editable,
        enabled: true,
        changed_at,
    }
}

/// The kinds it is offered to. One this version does not know is dropped.
fn kinds(said: Option<&str>) -> Vec<String> {
    let named = listed(said).into_iter().map(|one| one.to_ascii_lowercase());
    named
        .filter(|one| SessionKind::NAMES.contains(&one.as_str()))
        .collect()
}
