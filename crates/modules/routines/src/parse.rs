//! What a routine file says: its front matter, one `key: value` line a field, then the words.

use groove_types::front_matter::{field, listed, split};
use groove_types::{Action, Routine, RoutineKind, Trigger};

/// One routine from its file, or why the file is no routine.
pub fn parse(id: &str, name: &str, text: &str) -> Result<Routine, String> {
    let (front, words) = split(text).ok_or("it opens with no `---` front matter")?;
    let field = |key: &str| field(front, key);
    let skills: Vec<String> = listed(field("skills").as_deref());
    let words = words.trim().to_string();
    let kind = match field("kind").as_deref() {
        Some("bound") => RoutineKind::Bound,
        Some("standalone") => RoutineKind::Standalone,
        Some("action") => RoutineKind::Action,
        _ => return Err("its `kind` is neither `bound`, `standalone` nor `action`".into()),
    };
    let action = acted(kind, field("do"))?;
    if kind != RoutineKind::Action {
        asked(&skills, &words)?;
    }
    let on = listed(field("on").as_deref())
        .into_iter()
        .map(|word| Trigger::named(&word).ok_or(format!("`{word}` is no trigger")))
        .collect::<Result<Vec<_>, _>>()?;
    checked(kind, &on)?;
    Ok(Routine {
        id: id.to_string(),
        name: name.to_string(),
        description: field("description").unwrap_or_default(),
        skills,
        kind,
        on,
        words,
        action,
    })
}

/// An agent's routine says what to do: one skill, or words.
fn asked(skills: &[String], words: &str) -> Result<(), String> {
    if skills.is_empty() && words.is_empty() {
        return Err("it names no `skills` and asks nothing under its header".into());
    }
    if skills.len() > 1 && words.is_empty() {
        return Err("with several `skills`, the words under its header say what to do".into());
    }
    Ok(())
}

/// An action routine names one action Groove knows in `do`; no other kind has one.
fn acted(kind: RoutineKind, said: Option<String>) -> Result<Option<Action>, String> {
    match (kind, said) {
        (RoutineKind::Action, None) => Err("an action routine names what it does in `do`".into()),
        (RoutineKind::Action, Some(word)) => {
            let known: Vec<&str> = Action::ALL.iter().map(|one| one.name()).collect();
            let why = format!("`{word}` is no action: {}", known.join(", "));
            Action::named(&word).map(Some).ok_or(why)
        }
        (_, Some(_)) => Err("only an action routine has a `do`".into()),
        (_, None) => Ok(None),
    }
}

/// A bound routine answers to a session's events; the other kinds not.
fn checked(kind: RoutineKind, on: &[Trigger]) -> Result<(), String> {
    match kind {
        RoutineKind::Bound => {
            if on.is_empty() {
                return Err("a bound routine answers to at least one event of a session".into());
            }
            if let Some(one) = on.iter().find(|one| !one.of_a_session()) {
                return Err(format!("`{}` is no event of a session", one.name()));
            }
        }
        RoutineKind::Standalone | RoutineKind::Action => {
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
