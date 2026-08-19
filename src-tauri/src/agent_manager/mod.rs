//! What runs inside a PTY: the Claude agent, the session terminal, the setup
//! shell. The PTY mechanics live in `core::pty`.

use crate::core::pty::{PtySpec, Ptys};

/// Every in-app terminal runs bash, deliberately not $SHELL: one shell means one
/// redraw behavior and one escape-sequence dialect against xterm.js, on every
/// launch method.
const TERMINAL_SHELL: &str = "/bin/bash";

/// Overall MCP tool-call cap for spawned agents (24h ≈ "no timeout"): a write op
/// waits for the user's approval, which they may leave queued for as long as they
/// like.
const MCP_TOOL_TIMEOUT_MS: &str = "86400000";

/// Write ops block on a human approval that can be deferred indefinitely (Esc
/// parks it in the queue for later review), so the agent's MCP call must be
/// allowed to wait. With the CLI defaults the call times out and the agent
/// RETRIES, queueing a duplicate of an action the user hasn't decided on yet.
/// `CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT=0` disables the idle timeout (the CLI
/// documents 0 as "disabled"); MCP_TOOL_TIMEOUT is the overall cap. Set on every
/// PTY so `claude` behaves the same started from an in-app terminal.
fn claude_env() -> Vec<(&'static str, String)> {
    vec![
        ("CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT", "0".to_string()),
        ("MCP_TOOL_TIMEOUT", MCP_TOOL_TIMEOUT_MS.to_string()),
    ]
}

// ─── Session identity ───────────────────────────────────────────────────────

/// Fixed namespace for deriving deterministic per-task Claude session UUIDs.
/// (A constant random UUID — its only job is to namespace `new_v5`.)
const SESSION_NS: uuid::Uuid = uuid::uuid!("6f3d8a1c-2b7e-4f5a-9c0d-1e2f3a4b5c6d");

/// The Claude session UUID for a task is derived deterministically from its id,
/// so we never have to guess which session file belongs to the task: the same
/// task_id always maps to the same UUID, on every launch, with no persistence.
pub fn task_session_uuid(task_id: &str) -> String {
    uuid::Uuid::new_v5(&SESSION_NS, task_id.as_bytes()).to_string()
}

/// Legacy: the per-task `.agent_session_id` file written by the old watcher
/// approach. Read-only now — honored as a fallback so existing conversations
/// keep resuming, but never written.
fn load_legacy_session_id(task_id: &str) -> Option<String> {
    std::fs::read_to_string(crate::worktrees::session_dir(task_id).join(".agent_session_id"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Encode a CWD path the same way Claude Code does for its session directory.
fn claude_projects_dir(cwd: &str) -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let encoded = cwd.trim_end_matches('/').replace('/', "-");
    std::path::PathBuf::from(&home)
        .join(".claude")
        .join("projects")
        .join(encoded)
}

/// True if Claude already has a persisted session file for this UUID under `cwd`.
fn session_exists(cwd: &str, uuid: &str) -> bool {
    claude_projects_dir(cwd).join(format!("{uuid}.jsonl")).is_file()
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Return the absolute path to the `claude` binary.
/// First tries $HOME/.local/bin/claude (most common npm global install),
/// then falls back to `which` (works when Tauri inherits the user's PATH).
pub(crate) fn resolve_claude_bin() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let local = format!("{home}/.local/bin/claude");
    if std::path::Path::new(&local).exists() {
        return local;
    }
    for p in ["/usr/local/bin/claude", "/usr/bin/claude"] {
        if std::path::Path::new(p).exists() {
            return p.to_string();
        }
    }
    "claude".to_string()
}

/// The worktree root (from config, tilde-expanded), falling back to $HOME if it
/// isn't a directory. Used as the cwd for the agent and for terminals that have
/// no worktree yet (e.g. a fresh explorer with no repos added).
fn resolve_root_cwd() -> String {
    let raw = crate::core::config::get()
        .map(|c| c.git.worktree_root)
        .unwrap_or_else(|| std::env::var("HOME").unwrap_or_default());
    let cwd = crate::core::fs::expand_tilde(&raw);
    if std::path::Path::new(&cwd).is_dir() {
        cwd
    } else {
        std::env::var("HOME").unwrap_or_default()
    }
}

// ─── IPC commands ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn start_agent_session(
    app: tauri::AppHandle,
    task_id: String,
    ptys: tauri::State<'_, Ptys>,
) -> Result<String, String> {
    let cwd = resolve_root_cwd();

    // Each task owns one deterministic Claude session UUID. We pick the id
    // ourselves instead of discovering it after the fact, so a task always maps
    // to the same conversation and the interactive resume picker never appears.
    //
    // - First launch (no session file yet) → create with our chosen `--session-id`.
    // - Subsequent launches (file exists)  → `--resume` that exact session.
    // A legacy `.agent_session_id` (from the old watcher approach) is honored as
    // a fallback when its session file still exists, so old conversations resume.
    let uuid = task_session_uuid(&task_id);
    let mut args: Vec<String> = if session_exists(&cwd, &uuid) {
        vec!["--resume".to_string(), uuid]
    } else if let Some(legacy) = load_legacy_session_id(&task_id).filter(|id| session_exists(&cwd, id)) {
        vec!["--resume".to_string(), legacy]
    } else {
        vec!["--session-id".to_string(), uuid]
    };

    // Bind THIS agent's MCP connection to THIS task.
    //
    // Every agent runs at the same worktree root, so they'd otherwise share one
    // MCP config and resolve "the active task" through the UI's focus — meaning
    // an agent working on task A starts answering about task B the moment the
    // user looks elsewhere. The `?task=` query param makes the server bind the
    // connection to this task for its lifetime instead.
    //
    // `--mcp-config` takes inline JSON, and WITHOUT `--strict-mcp-config` the
    // user's other servers (notion, kubernetes, …) still load from disk.
    args.push("--mcp-config".to_string());
    args.push(
        serde_json::json!({
            "mcpServers": {
                // The server name is the agent's tool PREFIX
                // (mcp__groove__get_task_diff). Renaming it invalidates any
                // permission allowlist or hook matcher built on the old prefix.
                "groove": {
                    "type": "sse",
                    "url": crate::mcp_server::sse_url(&task_id),
                }
            }
        })
        .to_string(),
    );

    // Report this agent's state (working / waiting on the user / idle) back to the
    // app. See agent_hooks for why hooks rather than reading the terminal.
    args.push("--settings".to_string());
    args.push(hook_settings(&task_id));

    // Drop the reported activity with the process, so a "waiting on you" can
    // never outlive the agent it described.
    let app_exit = app.clone();
    let task_exit = task_id.clone();
    let on_exit = Box::new(move || {
        use tauri::Manager;
        crate::agent_hooks::forget(
            &app_exit.state::<crate::agent_hooks::ActivityState>(),
            &task_exit,
        );
    });

    ptys.spawn(
        &app,
        PtySpec {
            task_id,
            kind: "agent",
            cwd,
            program: resolve_claude_bin(),
            args,
            env: claude_env(),
            on_exit: Some(on_exit),
        },
    )
    .map_err(|e| e.to_string())
}

/// Inline `--settings` JSON wiring Claude Code's lifecycle hooks to our loopback
/// server. Verified with Claude Code 2.1.220: inline settings MERGE with the
/// user's own settings files, so their hooks keep running alongside these.
///
/// `curl` is best-effort by design — `-m 2` caps the stall if the app is gone,
/// and a failing hook must never hold up the agent, only cost a status update.
fn hook_settings(task_id: &str) -> String {
    let command = format!(
        "curl -s -m 2 -X POST -H 'content-type: application/json' --data-binary @- '{}' >/dev/null 2>&1 || true",
        crate::mcp_server::hook_url(task_id)
    );
    let post = serde_json::json!([{ "hooks": [{ "type": "command", "command": command }] }]);
    serde_json::json!({
        "hooks": {
            // Session is up, between turns.
            "SessionStart": post,
            // A prompt was submitted (by the user or the pill) — turn started.
            "UserPromptSubmit": post,
            // Tool about to run; also the tool a following Notification is about.
            "PreToolUse": post,
            // Tool finished, so nothing is pending approval.
            "PostToolUse": post,
            // Claude wants the user (permission prompt, idle nudge).
            "Notification": post,
            // Turn finished; payload carries last_assistant_message.
            "Stop": post,
        }
    })
    .to_string()
}

/// A shell for the setup screen's sign-in, for the user to run the login in.
///
/// A shell rather than `<tool> auth login` itself: a self-hosted GitLab needs
/// `--hostname`, and there is no way to ask for every flag a forge CLI accepts. So
/// the modal names the command and the user runs it, with edits.
///
/// Not tied to a task — it exists before any task does. The session row uses a
/// synthetic id so the reaper cleans it up like any other PTY when the shell exits.
pub(crate) fn start_login_pty(
    app: &tauri::AppHandle,
    cwd: &str,
    ptys: &Ptys,
) -> anyhow::Result<String> {
    ptys.spawn(
        app,
        PtySpec {
            task_id: "__auth__".to_string(),
            kind: "auth",
            cwd: cwd.to_string(),
            program: TERMINAL_SHELL.to_string(),
            args: vec![],
            env: claude_env(),
            on_exit: None,
        },
    )
}

#[tauri::command]
pub async fn start_terminal_session(
    app: tauri::AppHandle,
    task_id: String,
    worktree_path: Option<String>,
    ptys: tauri::State<'_, Ptys>,
) -> Result<String, String> {
    // Open in the worktree when there is one; otherwise fall back to the worktree
    // root (an explorer with no repos added yet).
    let cwd = match worktree_path.as_deref() {
        Some(p) if !p.is_empty() && std::path::Path::new(p).is_dir() => p.to_string(),
        _ => resolve_root_cwd(),
    };
    ptys.spawn(
        &app,
        PtySpec {
            task_id,
            kind: "terminal",
            cwd,
            program: TERMINAL_SHELL.to_string(),
            args: vec![],
            env: claude_env(),
            on_exit: None,
        },
    )
    .map_err(|e| e.to_string())
}
