use std::{
    collections::HashMap,
    io::Write,
    sync::{Arc, Mutex},
};

use sqlx::SqlitePool;
use tauri::Emitter;

/// Overall MCP tool-call cap for spawned agents (24h ≈ "no timeout"): a write op
/// waits for the user's approval, which they may leave queued for as long as they
/// like. See the env setup in `start_pty_session`.
const MCP_TOOL_TIMEOUT_MS: &str = "86400000";

// SAFETY: PTY master fd (TIOCSWINSZ ioctl) is safe to call from any thread on Unix.
struct SendableMasterPty(Box<dyn portable_pty::MasterPty>);
unsafe impl Send for SendableMasterPty {}

// ─── PTY session ──────────────────────────────────────────────────────────────

struct PtySession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Arc<Mutex<SendableMasterPty>>,
    pid: Option<u32>,
}

// ─── Module state ─────────────────────────────────────────────────────────────

pub struct State {
    inner: Arc<StateInner>,
}

struct StateInner {
    sessions: Mutex<HashMap<String, PtySession>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(StateInner {
                sessions: Mutex::new(HashMap::new()),
            }),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
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
fn legacy_session_id_file(task_id: &str) -> std::path::PathBuf {
    crate::git_engine::session_dir(task_id).join(".agent_session_id")
}

fn load_legacy_session_id(task_id: &str) -> Option<String> {
    std::fs::read_to_string(legacy_session_id_file(task_id))
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

/// Tell the child what terminal it is talking to.
///
/// The front end is xterm.js, so this is the truth and never the launcher's idea of
/// it: a desktop launch has no `TERM` at all, and a program with no terminfo cannot
/// move the cursor. zsh then redraws its line by appending — the syntax highlighter
/// rewrites the character it just inserted instead of overwriting it, so the first
/// keystroke of every command appeared twice and everything after it drifted a
/// column. Inheriting `TERM` is just as wrong: a shell told it is inside tmux
/// writes sequences xterm.js does not implement.
fn describe_terminal(cmd: &mut portable_pty::CommandBuilder) {
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
}

/// Expand a leading `~` to $HOME. portable_pty does not run a shell so `~` is
/// never expanded and chdir("~/…") fails with ENOENT, exiting the child immediately.
pub(crate) fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/{rest}")
    } else if path == "~" {
        std::env::var("HOME").unwrap_or_default()
    } else {
        path.to_string()
    }
}

/// Return the absolute path to the `claude` binary.
/// First tries $HOME/.local/bin/claude (most common npm global install),
/// then falls back to `which` (works when Tauri inherits the user's PATH).
pub(crate) fn resolve_claude_bin() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let local = format!("{home}/.local/bin/claude");
    if std::path::Path::new(&local).exists() {
        return local;
    }
    // Try common system-wide locations
    for p in ["/usr/local/bin/claude", "/usr/bin/claude"] {
        if std::path::Path::new(p).exists() {
            return p.to_string();
        }
    }
    // Last resort: let the OS resolve it from PATH (works when PATH is inherited)
    "claude".to_string()
}

/// The worktree root (from config, tilde-expanded), falling back to $HOME if it
/// isn't a directory. Used as the cwd for the agent and for terminals that have
/// no worktree yet (e.g. a fresh explorer with no repos added).
fn resolve_root_cwd(task_state: &crate::task_manager::State) -> String {
    let raw = task_state
        .get_config()
        .map(|c| c.git.worktree_root)
        .unwrap_or_else(|| std::env::var("HOME").unwrap_or_default());
    let cwd = expand_tilde(&raw);
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
    agent_state: tauri::State<'_, State>,
    task_state: tauri::State<'_, crate::task_manager::State>,
) -> Result<String, String> {
    let cwd = resolve_root_cwd(&task_state);

    let claude_bin = resolve_claude_bin();

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

    start_pty_session(&app, &task_id, &cwd, "agent", &claude_bin, &args, &agent_state)
        .await
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
pub(crate) async fn start_login_pty(
    app: &tauri::AppHandle,
    cwd: &str,
    agent_state: &State,
) -> anyhow::Result<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    start_pty_session(app, "__auth__", cwd, "auth", &shell, &[], agent_state).await
}

#[tauri::command]
pub async fn start_terminal_session(
    app: tauri::AppHandle,
    task_id: String,
    worktree_path: Option<String>,
    agent_state: tauri::State<'_, State>,
    task_state: tauri::State<'_, crate::task_manager::State>,
) -> Result<String, String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    // Open in the worktree when there is one; otherwise fall back to the worktree
    // root (an explorer with no repos added yet).
    let cwd = match worktree_path.as_deref() {
        Some(p) if !p.is_empty() && std::path::Path::new(p).is_dir() => p.to_string(),
        _ => resolve_root_cwd(&task_state),
    };
    start_pty_session(&app, &task_id, &cwd, "terminal", &shell, &[], &agent_state)
        .await
        .map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
async fn start_pty_session(
    app: &tauri::AppHandle,
    task_id: &str,
    worktree_path: &str,
    pty_type: &str,
    program: &str,
    extra_args: &[String],
    agent_state: &State,
) -> anyhow::Result<String> {
    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system
        // The conventional default. The frontend resizes to the real geometry as
        // soon as the host attaches; until then a shell that wraps at 80 is far
        // less wrong than one that wraps at 120.
        .openpty(portable_pty::PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| anyhow::anyhow!("openpty failed: {e}"))?;

    let mut cmd = portable_pty::CommandBuilder::new(program);
    for arg in extra_args {
        cmd.arg(arg);
    }
    cmd.cwd(worktree_path);
    describe_terminal(&mut cmd);

    // Write ops block on a human approval that can be deferred indefinitely (Esc
    // parks it in the queue for later review), so the agent's MCP call must be
    // allowed to wait. With the CLI defaults the call times out and the agent
    // RETRIES, queueing a duplicate of an action the user hasn't decided on yet.
    // `CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT=0` disables the idle timeout (the CLI
    // documents 0 as "disabled"); MCP_TOOL_TIMEOUT is the overall cap.
    cmd.env("CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT", "0");
    cmd.env("MCP_TOOL_TIMEOUT", MCP_TOOL_TIMEOUT_MS);

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| anyhow::anyhow!("spawn failed: {e}"))?;

    let pid = child.process_id();

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| anyhow::anyhow!("take_writer failed: {e}"))?;

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| anyhow::anyhow!("clone_reader failed: {e}"))?;

    let session_id = uuid::Uuid::new_v4().to_string();

    // Spawn background reader that forwards PTY output to the frontend. It owns the
    // child handle so it can `wait()` on natural exit (avoiding a zombie), then it
    // reaps the session from the in-memory map.
    let app_clone = app.clone();
    let sid_clone = session_id.clone();
    let state_inner = Arc::clone(&agent_state.inner);
    let is_agent = pty_type == "agent";
    let task_for_reaper = task_id.to_string();
    tokio::task::spawn_blocking(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match std::io::Read::read(&mut reader, &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let data = &buf[..n];
                    crate::pty_trace::record("pty<<", &sid_clone, data);
                    // Base64, not a JSON array of numbers. This is the busiest path in
                    // the app — an agent redraws its whole screen as it thinks — and a
                    // number array costs about four JSON characters per byte, every one
                    // of them parsed individually on the UI thread. Measured on the
                    // same 2 MB of output: 14.7 KB per 4 KB chunk against 5.5 KB, and
                    // 37 ms of parsing against 9 ms.
                    let _ = app_clone.emit(
                        crate::events::PTY_OUTPUT,
                        serde_json::json!({
                            "session_id": sid_clone,
                            "b64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data),
                        }),
                    );
                }
            }
        }
        // Reader hit EOF → the PTY exited on its own. Reap the child so it doesn't
        // linger as a zombie, then drop the session entry.
        let _ = child.wait();
        if let Ok(mut sessions) = state_inner.sessions.lock() {
            sessions.remove(&sid_clone);
        }
        // Drop the reported activity with the process, so a "waiting on you" can
        // never outlive the agent it described.
        if is_agent {
            use tauri::Manager;
            crate::agent_hooks::forget(
                &app_clone.state::<crate::agent_hooks::ActivityState>(),
                &task_for_reaper,
            );
        }
        let _ = app_clone.emit(crate::events::PTY_EXIT, serde_json::json!({ "session_id": sid_clone }));
    });

    let master = Arc::new(Mutex::new(SendableMasterPty(pair.master)));

    let session = PtySession {
        writer: Arc::new(Mutex::new(writer)),
        master,
        pid,
    };

    if let Ok(mut sessions) = agent_state.inner.sessions.lock() {
        sessions.insert(session_id.clone(), session);
    }

    app.emit(
        crate::events::PTY_STARTED,
        serde_json::json!({ "session_id": session_id, "task_id": task_id, "pty_type": pty_type }),
    )?;

    Ok(session_id)
}

#[tauri::command]
pub async fn stop_agent_session(
    app: tauri::AppHandle,
    session_id: String,
    agent_state: tauri::State<'_, State>,
) -> Result<(), String> {
    if let Ok(mut sessions) = agent_state.inner.sessions.lock() {
        if let Some(session) = sessions.remove(&session_id) {
            // Send SIGTERM to the process
            if let Some(pid) = session.pid {
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .status();
            }
        }
    }

    app.emit(crate::events::PTY_EXIT, serde_json::json!({ "session_id": session_id }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn write_pty(
    session_id: String,
    data: Vec<u8>,
    agent_state: tauri::State<'_, State>,
) -> Result<(), String> {
    let writer = agent_state
        .inner
        .sessions
        .lock()
        .ok()
        .and_then(|sessions| {
            sessions
                .get(&session_id)
                .map(|s| Arc::clone(&s.writer))
        })
        .ok_or_else(|| format!("session {session_id} not found"))?;

    crate::pty_trace::record("pty>>", &session_id, &data);

    // The write can block (PTY buffer full); do it off the async runtime, locking
    // the std Mutex inside the blocking closure rather than across the .await.
    tokio::task::spawn_blocking(move || {
        writer
            .lock()
            .map_err(|_| "writer lock poisoned".to_string())?
            .write_all(&data)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(())
}

#[tauri::command]
pub async fn resize_pty(
    session_id: String,
    rows: u16,
    cols: u16,
    agent_state: tauri::State<'_, State>,
) -> Result<(), String> {
    let master_arc = agent_state
        .inner
        .sessions
        .lock()
        .ok()
        .and_then(|s| s.get(&session_id).map(|s| Arc::clone(&s.master)))
        .ok_or_else(|| format!("session {session_id} not found"))?;
    let guard = master_arc.lock().map_err(|_| "lock poisoned".to_string())?;
    guard
        .0
        .resize(portable_pty::PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())
}

// ─── Confirmation bridge IPC (exposed here per architecture IPC surface) ──────

#[tauri::command]
pub async fn resolve_confirmation(
    id: String,
    approved: bool,
    payload_overrides: Option<serde_json::Value>,
    pool: tauri::State<'_, SqlitePool>,
    bridge: tauri::State<'_, crate::confirmation_bridge::Bridge>,
) -> Result<(), String> {
    bridge
        .resolve(&pool, &id, approved, payload_overrides)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // A PTY child with no TERM cannot move its cursor, and zsh's line redraw then
    // duplicated the first character of every command. The value has to be one
    // xterm.js actually implements, and it has to be SET rather than inherited.
    #[test]
    fn pty_children_are_told_they_are_an_xterm() {
        let mut cmd = portable_pty::CommandBuilder::new("zsh");
        describe_terminal(&mut cmd);
        assert_eq!(cmd.get_env("TERM").unwrap(), "xterm-256color");
        assert_eq!(cmd.get_env("COLORTERM").unwrap(), "truecolor");
    }
}
