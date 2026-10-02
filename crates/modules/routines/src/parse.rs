//! What a routine file says: its front matter, one `key: value` line a field, then the words.

use groove_types::{Routine, RoutineKind, Trigger};

/// What a bound routine may do without asking.
const BOUND_SCOPE: [&str; 5] = ["edit", "commit", "push", "comment", "mr"];

/// One routine from its file, or why the file is no routine.
pub fn parse(id: &str, name: &str, text: &str) -> Result<Routine, String> {
    let (front, words) = split(text).ok_or("it opens with no `---` front matter")?;
    let field = |key: &str| field(front, key);
    let skills: Vec<String> = listed(field("skills")).collect();
    let words = words.trim().to_string();
    if skills.is_empty() && words.is_empty() {
        return Err("it names no `skills` and asks nothing under its header".into());
    }
    if skills.len() > 1 && words.is_empty() {
        return Err("with several `skills`, the words under its header say what to do".into());
    }
    let kind = match field("kind").as_deref() {
        Some("bound") => RoutineKind::Bound,
        Some("standalone") => RoutineKind::Standalone,
        _ => return Err("its `kind` is neither `bound` nor `standalone`".into()),
    };
    let on = listed(field("on"))
        .map(|word| Trigger::named(&word).ok_or(format!("`{word}` is no trigger")))
        .collect::<Result<Vec<_>, _>>()?;
    let scope: Vec<String> = listed(field("scope")).collect();
    checked(kind, &on, &scope)?;
    Ok(Routine {
        id: id.to_string(),
        name: name.to_string(),
        description: field("description").unwrap_or_default(),
        skills,
        kind,
        on,
        scope,
        words,
    })
}

/// A bound routine answers to a session's events and writes what it holds; a standalone one not.
fn checked(kind: RoutineKind, on: &[Trigger], scope: &[String]) -> Result<(), String> {
    match kind {
        RoutineKind::Bound => {
            if on.is_empty() {
                return Err("a bound routine answers to at least one event of a session".into());
            }
            if let Some(one) = on.iter().find(|one| !one.of_a_session()) {
                return Err(format!("`{}` is no event of a session", one.name()));
            }
            if let Some(one) = scope
                .iter()
                .find(|one| !BOUND_SCOPE.contains(&one.as_str()))
            {
                let known = BOUND_SCOPE.join(", ");
                return Err(format!("`{one}` is not in a bound scope: {known}"));
            }
        }
        RoutineKind::Standalone => {
            if let Some(one) = on.iter().find(|one| one.of_a_session()) {
                return Err(format!(
                    "`{}` is about a session: make it bound",
                    one.name()
                ));
            }
        }
    }
    Ok(())
}

/// The front matter and what follows it.
fn split(text: &str) -> Option<(&str, &str)> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---")?;
    let after = &rest[end + 4..];
    Some((
        &rest[..end],
        after.split_once('\n').map_or("", |(_, words)| words),
    ))
}

fn field(front: &str, key: &str) -> Option<String> {
    front.lines().find_map(|line| {
        let (named, value) = line.split_once(':')?;
        let value = value.trim().trim_matches(['"', '\'']).trim();
        (named.trim() == key && !value.is_empty()).then(|| value.to_string())
    })
}

fn listed(said: Option<String>) -> impl Iterator<Item = String> {
    let words: Vec<String> = said
        .iter()
        .flat_map(|one| one.split(','))
        .map(|one| one.trim().to_string())
        .filter(|one| !one.is_empty())
        .collect();
    words.into_iter()
}
