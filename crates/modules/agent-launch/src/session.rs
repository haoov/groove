use std::path::{Path, PathBuf};

use groove_types::Session;

use crate::Paths;

/// Namespace for the per-session Claude UUIDs. Changing it orphans every resumable session.
const NAMESPACE: uuid::Uuid = uuid::uuid!("6f3d8a1c-2b7e-4f5a-9c0d-1e2f3a4b5c6d");

/// One Claude session per Groove session, derived from its id.
pub fn session_uuid(session_id: &str) -> String {
    uuid::Uuid::new_v5(&NAMESPACE, session_id.as_bytes()).to_string()
}

/// `--session-id` on the first launch, `--resume` once Claude has a file for it.
pub(crate) fn identity_args(session: &Session, paths: &Paths<'_>) -> Vec<String> {
    let uuid = session_uuid(session.id.as_str());
    let flag = if session_file(paths.home, paths.cwd, &uuid).is_file() {
        "--resume"
    } else {
        "--session-id"
    };
    vec![flag.to_string(), uuid]
}

/// `~/.claude/projects/<cwd with every non-alphanumeric as ->/<uuid>.jsonl`.
pub(crate) fn session_file(home: &Path, cwd: &Path, uuid: &str) -> PathBuf {
    let encoded: String = cwd
        .to_string_lossy()
        .trim_end_matches('/')
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    home.join(".claude")
        .join("projects")
        .join(encoded)
        .join(format!("{uuid}.jsonl"))
}
