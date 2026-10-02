//! What a `SKILL.md` says about itself: its front matter, one line a field.

use groove_types::{Skill, Timestamp};

/// One skill as its file describes it; its directory is its identity, never `name:`.
pub(crate) fn skill(
    plugin: &str,
    name: &str,
    body: &str,
    editable: bool,
    changed_at: Timestamp,
) -> Skill {
    let front = front_matter(body);
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

/// What stands between the opening `---` and the next one.
fn front_matter(body: &str) -> &str {
    let rest = body
        .strip_prefix("---\n")
        .or_else(|| body.strip_prefix("---\r\n"))
        .unwrap_or("");
    match rest.find("\n---") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

/// One `key: value` line, split on its first colon.
fn field(front: &str, key: &str) -> Option<String> {
    front.lines().find_map(|line| {
        let (named, value) = line.split_once(':')?;
        if named.trim() != key {
            return None;
        }
        let value = value.trim().trim_matches(['"', '\'']).trim();
        (!value.is_empty()).then(|| value.to_string())
    })
}

/// The kinds it is offered to. One this version does not know is dropped.
fn kinds(said: Option<&str>) -> Vec<String> {
    const KNOWN: [&str; 3] = ["task", "explorer", "review"];
    said.into_iter()
        .flat_map(|one| one.split(','))
        .map(|one| one.trim().to_ascii_lowercase())
        .filter(|one| KNOWN.contains(&one.as_str()))
        .collect()
}
