//! What runs inside a PTY: the Claude agent, the session terminal, the setup
//! shell. The PTY mechanics live in `core::pty`.

use crate::core::error::{AppError, AppResult, ErrorKind};
use crate::core::pty::{PtySpec, Ptys};

/// Every in-app terminal runs bash, not $SHELL: one escape-sequence dialect for xterm.js.
const TERMINAL_SHELL: &str = "/bin/bash";

/// MCP tool-call cap: 24h.
const MCP_TOOL_TIMEOUT_MS: &str = "86400000";

/// No MCP idle timeout and a 24h cap. A timed-out gated write is retried and queues
/// a duplicate. Set on every PTY.
fn claude_env() -> Vec<(&'static str, String)> {
    vec![
        ("CLAUDE_CODE_MCP_TOOL_IDLE_TIMEOUT", "0".to_string()),
        ("MCP_TOOL_TIMEOUT", MCP_TOOL_TIMEOUT_MS.to_string()),
    ]
}

// ─── Session identity ───────────────────────────────────────────────────────

/// Namespace for deterministic per-task Claude session UUIDs.
const SESSION_NS: uuid::Uuid = uuid::uuid!("6f3d8a1c-2b7e-4f5a-9c0d-1e2f3a4b5c6d");

/// The Claude session UUID for a task, derived from its id.
pub fn task_session_uuid(task_id: &str) -> String {
    uuid::Uuid::new_v5(&SESSION_NS, task_id.as_bytes()).to_string()
}

/// Fallback session id from `.agent_session_id`. Read only, never written.
fn load_legacy_session_id(task_id: &str) -> Option<String> {
    std::fs::read_to_string(crate::worktrees::session_dir(task_id).join(".agent_session_id"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Encode a cwd the way Claude Code names its session directory: every
/// non-alphanumeric character becomes `-`.
fn claude_projects_dir(cwd: &str) -> std::path::PathBuf {
    let encoded: String = cwd
        .trim_end_matches('/')
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let home = std::env::var("HOME").unwrap_or_default();
    std::path::PathBuf::from(&home)
        .join(".claude")
        .join("projects")
        .join(encoded)
}

/// True if Claude already has a persisted session file for this UUID under `cwd`.
fn session_exists(cwd: &str, uuid: &str) -> bool {
    claude_projects_dir(cwd)
        .join(format!("{uuid}.jsonl"))
        .is_file()
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Absolute path to the `claude` binary.
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

#[cfg(test)]
mod tests {
    /// Pinned to a real `~/.claude/projects` entry: every non-alphanumeric char becomes `-`.
    #[test]
    fn projects_dir_encodes_like_claude_code() {
        let dir = super::claude_projects_dir("/home/x/worktrees/gitlab.wiremind.io/devops/");
        assert!(dir
            .to_string_lossy()
            .ends_with("/.claude/projects/-home-x-worktrees-gitlab-wiremind-io-devops"));
    }

    #[test]
    fn the_hook_token_stays_out_of_the_command_line() {
        let token = crate::mcp_server::auth::token();
        let settings = super::hook_settings("t-1", "/tmp/t-1.hooks.curl");
        assert!(
            !settings.contains(token),
            "the token reaches /proc/<pid>/cmdline: {settings}"
        );
        assert!(settings.contains("-K '/tmp/t-1.hooks.curl'"));
        assert!(super::hook_curl_config().contains(token));
    }
}

/// The worktree root from config, or $HOME when it is not a directory.
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
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> AppResult<String> {
    let cwd = resolve_root_cwd();

    // One Claude session per task: created on first launch, resumed after.
    let uuid = task_session_uuid(&task_id);
    let mut args: Vec<String> = if session_exists(&cwd, &uuid) {
        vec!["--resume".to_string(), uuid]
    } else if let Some(legacy) =
        load_legacy_session_id(&task_id).filter(|id| session_exists(&cwd, id))
    {
        vec!["--resume".to_string(), legacy]
    } else {
        vec!["--session-id".to_string(), uuid]
    };

    // Both blobs go through files, not argv: they carry the bearer token.
    // No `--strict-mcp-config`: the user's own servers still load.
    let mcp_config = serde_json::json!({
        "mcpServers": {
            // The server name is the agent's tool prefix (mcp__groove__*).
            "groove": {
                "type": "sse",
                "url": crate::mcp_server::sse_url(&task_id),
                "headers": {
                    "Authorization": format!("Bearer {}", crate::mcp_server::auth::token()),
                }
            }
        }
    });
    args.push("--mcp-config".to_string());
    args.push(
        write_launch_file(&app, &task_id, "mcp.json", &mcp_config.to_string())
            .map_err(|e| AppError::from(e).with_kind(ErrorKind::Agent))?,
    );

    // Hooks report the agent's state; see agent_hooks.
    let curl_config = write_launch_file(&app, &task_id, "hooks.curl", &hook_curl_config())
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Agent))?;
    args.push("--settings".to_string());
    args.push(
        write_launch_file(
            &app,
            &task_id,
            "settings.json",
            &hook_settings(&task_id, &curl_config),
        )
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Agent))?,
    );

    // The core prompt. `--append-system-prompt` is the inline fallback.
    let session = crate::core::db::store::sessions::get_opt(&*pool, &task_id)
        .await
        .ok()
        .flatten();
    args.push("--append-system-prompt-file".to_string());
    args.push(
        write_launch_file(
            &app,
            &task_id,
            "prompt.md",
            &crate::skills::core_prompt(&task_id, session.as_ref()),
        )
        .map_err(|e| AppError::from(e).with_kind(ErrorKind::Agent))?,
    );

    // Skills, per session: `--plugin-dir` is launch-scoped.
    for dir in crate::skills::plugin_dirs(&app) {
        args.push("--plugin-dir".to_string());
        args.push(dir);
    }

    // Drop the reported activity with the process.
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
    .map_err(|e| AppError::from(e).with_kind(ErrorKind::Agent))
}

/// Write one agent-launch file (0600; it carries the loopback token) and return its path.
fn write_launch_file(
    app: &tauri::AppHandle,
    task_id: &str,
    name: &str,
    contents: &str,
) -> anyhow::Result<String> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    use tauri::Manager;

    let dir = app.path().app_data_dir()?.join("agent-launch");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{task_id}.{name}"));
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)?;
    // `mode` applies on create only; an existing file keeps whatever it had.
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(contents.as_bytes())?;
    Ok(path.to_string_lossy().to_string())
}

/// The bearer header for the hook calls, as a curl config file. Never argv:
/// `/proc/<pid>/cmdline` is world-readable and a hook runs on every tool call.
fn hook_curl_config() -> String {
    format!(
        "header = \"authorization: Bearer {}\"\n",
        crate::mcp_server::auth::token()
    )
}

/// `--settings` wiring Claude Code's hooks to the loopback server. Keep `-m 2 ... || true`:
/// a failing hook must never hold up the agent.
fn hook_settings(task_id: &str, curl_config_path: &str) -> String {
    let command = format!(
        "curl -s -m 2 -K '{}' -X POST -H 'content-type: application/json' --data-binary @- '{}' >/dev/null 2>&1 || true",
        curl_config_path,
        crate::mcp_server::hook_url(task_id)
    );
    let post = serde_json::json!([{ "hooks": [{ "type": "command", "command": command }] }]);
    serde_json::json!({
        "hooks": {
            "SessionStart": post,
            "UserPromptSubmit": post,
            "PreToolUse": post,
            "PostToolUse": post,
            "Notification": post,
            // Payload carries last_assistant_message.
            "Stop": post,
        }
    })
    .to_string()
}

/// A shell for the setup screen's sign-in. Not tied to a task; the synthetic id
/// lets the reaper clean it up like any other PTY.
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
) -> AppResult<String> {
    // The worktree when there is one, else the worktree root.
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
    .map_err(|e| AppError::from(e).with_kind(ErrorKind::Agent))
}
