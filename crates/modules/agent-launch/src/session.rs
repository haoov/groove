//! The uuid a launch gives the agent, derived from the session.

use std::path::{Path, PathBuf};

use groove_types::Session;

use crate::Paths;

/// Namespace for the per-session Claude UUIDs. Changing it orphans every resumable session.
const NAMESPACE: uuid::Uuid = uuid::uuid!("6f3d8a1c-2b7e-4f5a-9c0d-1e2f3a4b5c6d");

/// One Claude session per Groove session, derived from its id.
pub fn session_uuid(session_id: &str) -> String {
    uuid::Uuid::new_v5(&NAMESPACE, session_id.as_bytes()).to_string()
}

/// The conversation a session carries on: the one an earlier session handed it, or its own.
pub fn thread_of(launch_dir: &Path, session_id: &str) -> String {
    let handed = launch_dir.join(format!("{session_id}.thread"));
    match std::fs::read_to_string(handed) {
        Ok(uuid) if !uuid.trim().is_empty() => uuid.trim().to_string(),
        _ => session_uuid(session_id),
    }
}

/// Hands `from`'s conversation to `to`, for `to`'s next launch to resume.
pub fn hand_over(launch_dir: &Path, from: &str, to: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(launch_dir)?;
    let uuid = thread_of(launch_dir, from);
    std::fs::write(launch_dir.join(format!("{to}.thread")), uuid)
}

/// A new conversation for the session's next launch, in place of the one it carried.
pub fn fresh(launch_dir: &Path, session_id: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(launch_dir)?;
    let uuid = uuid::Uuid::new_v4().to_string();
    std::fs::write(launch_dir.join(format!("{session_id}.thread")), uuid)
}

const NAMES: [&str; 5] = [
    "prompt.md",
    "settings.json",
    "hooks.curl",
    "mcp.json",
    "thread",
];

/// The session's launch files taken away; one already gone is no error.
pub fn forget(launch_dir: &Path, session_id: &str) -> std::io::Result<()> {
    for name in NAMES {
        match std::fs::remove_file(launch_dir.join(format!("{session_id}.{name}"))) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    Ok(())
}

/// `--session-id` on the first launch, `--resume` once Claude has a file for it.
pub(crate) fn identity_args(session: &Session, paths: &Paths<'_>) -> Vec<String> {
    let uuid = thread_of(paths.launch_dir, session.id.as_str());
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
