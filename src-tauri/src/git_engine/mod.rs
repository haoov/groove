mod base_ref;
mod blame;
mod commits;
mod diff;
mod ops;
mod parse;
mod status;
mod types;
mod watcher;
mod worktrees;

// Glob re-exports are required: tauri::generate_handler! looks up __cmd__* symbols
// at the same path as the function, so they must all be available at git_engine::*.
// Base-ref resolution is internal: nothing outside the crate asks for it.
pub(crate) use base_ref::*;
pub use blame::*;
pub use commits::*;
pub use diff::*;
pub use ops::*;
pub use status::*;
// The DTOs the commands return; other modules construct BranchSpec / read MainRepo.
pub use types::*;
pub use watcher::*;
pub use worktrees::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Run `git <args>` in `dir` off the async runtime and return its raw Output,
/// for callers that inspect status/stderr themselves. All git invocations in
/// this module go through here so they never block a tokio worker thread.
pub(crate) async fn run_git_output(dir: &str, args: &[&str]) -> anyhow::Result<std::process::Output> {
    let dir = dir.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let joined = args.join(" ");
    let detail = format!("git {joined}");
    crate::core::timing::timed("subprocess", detail, async move {
        tokio::task::spawn_blocking(move || {
            std::process::Command::new("git")
                .args(&args)
                // Force English, machine-stable output: several call sites match on
                // git's messages (e.g. "already exists" when a worktree is being
                // re-provisioned), and on a localized system git answers in the
                // user's language ("existe déjà"), silently breaking those checks.
                .env("LC_ALL", "C")
                .env("LANG", "C")
                .current_dir(&dir)
                .output()
        })
        .await?
        .map_err(|e| anyhow::anyhow!("failed to run git {joined}: {e}"))
    })
    .await
}

/// Run `git <args>` in `dir` off the async runtime, check the exit status, and
/// return stdout. A non-zero exit becomes an error carrying stderr.
pub(crate) async fn run_git(dir: &str, args: &[&str]) -> anyhow::Result<String> {
    let joined = args.join(" ");
    let output = run_git_output(dir, args).await?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git {joined} failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub struct State {
    pub(crate) inner: Arc<StateInner>,
}

pub(crate) struct StateInner {
    // worktree path → notify watcher, kept alive for the lifetime of the session.
    // Keyed by path so re-opening a session doesn't spawn duplicate watchers.
    pub(crate) watchers: Mutex<HashMap<String, notify::RecommendedWatcher>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StateInner {
                watchers: Mutex::new(HashMap::new()),
            }),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
