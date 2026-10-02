//! The core prompt every agent starts with.

use std::path::Path;

use groove_types::Session;

const CORE_PROMPT: &str = include_str!("prompt.md");
const KNOWLEDGE: &str = include_str!("knowledge.md");

/// The system prompt appended at every launch: identity, and the team's facts when there are some.
pub fn core_prompt(session: &Session, knowledge: Option<&Path>) -> String {
    let who = format!(
        "session {} ({}): \"{}\"",
        session.id,
        session.kind.name(),
        session.title
    );
    let mut out = CORE_PROMPT.replace("{{session}}", &who);
    if let Some(dir) = knowledge {
        out.push('\n');
        out.push_str(&KNOWLEDGE.replace("{{knowledge}}", &dir.to_string_lossy()));
    }
    out
}
