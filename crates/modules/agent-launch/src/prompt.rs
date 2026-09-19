//! The core prompt every agent starts with.

use groove_types::Session;

const CORE_PROMPT: &str = include_str!("prompt.md");

/// The system prompt appended at every launch. Identity only; repos and MRs drift.
pub fn core_prompt(session: &Session) -> String {
    let who = format!(
        "session {} ({}): \"{}\"",
        session.id,
        session.kind.name(),
        session.title
    );
    CORE_PROMPT.replace("{{session}}", &who)
}
