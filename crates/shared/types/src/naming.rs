use crate::Session;

/// Whether `branch` names this session: a task branch carries its id or its tag.
pub fn names_session(branch: &str, session: &Session, tag: Option<&str>) -> bool {
    let hay = branch.to_lowercase();
    if hay.contains(&session.id.as_str().to_lowercase()) {
        return true;
    }
    tag.is_some_and(|t| !t.is_empty() && hay.contains(&t.to_lowercase()))
}
