use crate::{Session, SessionId, SessionKind};

/// The branch an explorer works on: `explorer/` and its id's own part.
pub fn explorer_branch(id: &SessionId) -> String {
    format!("explorer/{}", id.as_str().trim_start_matches("explorer-"))
}

/// Whether `branch` is an explorer's.
pub fn is_explorer_branch(branch: &str) -> bool {
    branch.starts_with("explorer/")
}

/// Whether `branch` names this session: an explorer's own, or one that carries its id or its tag.
pub fn names_session(branch: &str, session: &Session, tag: Option<&str>) -> bool {
    if session.kind == SessionKind::Explorer {
        return branch == explorer_branch(&session.id);
    }
    let hay = branch.to_lowercase();
    if holds(&hay, &session.id.as_str().to_lowercase()) {
        return true;
    }
    tag.is_some_and(|t| !t.is_empty() && holds(&hay, &t.to_lowercase()))
}

/// Whether `needle` stands in `hay` with no letter or digit against it.
fn holds(hay: &str, needle: &str) -> bool {
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric());
    hay.match_indices(needle).any(|(at, _)| {
        let before = hay[..at].chars().next_back();
        let after = hay[at + needle.len()..].chars().next();
        !word(before) && !word(after)
    })
}
