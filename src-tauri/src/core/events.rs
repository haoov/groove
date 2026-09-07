//! Tauri event names for backend→frontend events.
//! Every name must match `src/shared/ipc/events.ts`.

// ── Workspace / task lifecycle ──────────────────────────────────────────────
pub const WORKSPACE_STUB: &str = "workspace_stub";
pub const WORKSPACE_READY: &str = "workspace_ready";
pub const TASK_PAUSED: &str = "task_paused";
pub const TASK_FINISHED: &str = "task_finished";
pub const EXPLORER_DISCARDED: &str = "explorer_discarded";

// ── Confirmation bridge ─────────────────────────────────────────────────────
pub const CONFIRMATION_REQUESTED: &str = "confirmation_requested";
pub const CONFIRMATION_RESOLVED: &str = "confirmation_resolved";

// ── Git / worktrees ─────────────────────────────────────────────────────────
pub const WORKTREE_CLOSED: &str = "worktree_closed";
pub const REBASE_DONE: &str = "rebase_done";
pub const REBASE_CONFLICT: &str = "rebase_conflict";

// ── Annotations ─────────────────────────────────────────────────────────────
pub const ANNOTATION_RESOLVED: &str = "annotation_resolved";
/// An annotation the agent created.
pub const ANNOTATION_CREATED: &str = "annotation_created";
/// An annotation body the agent rewrote.
pub const ANNOTATION_UPDATED: &str = "annotation_updated";

// ── PTY (agent / terminal) ──────────────────────────────────────────────────
pub const PTY_STARTED: &str = "pty_started";
pub const PTY_OUTPUT: &str = "pty_output";
pub const PTY_EXIT: &str = "pty_exit";

// ── Agent activity (from Claude Code hooks) ─────────────────────────────────
/// One agent changed state: idle / working / waiting on the user.
pub const AGENT_ACTIVITY: &str = "agent_activity";

// ── Backend notices ─────────────────────────────────────────────────────────
/// A warning from background work; payload mirrors the frontend's `NotificationInput`.
pub const BACKEND_NOTICE: &str = "backend_notice";

use std::sync::OnceLock;

/// The app handle for modules with no `AppHandle`/`State` access. Set once during init.
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

pub fn set_app(handle: tauri::AppHandle) {
    let _ = APP.set(handle);
}

/// Emit a backend notice. No-op before `set_app`.
pub fn notice(
    kind: &str,
    source: &str,
    title: String,
    detail: Option<String>,
    task_id: Option<&str>,
) {
    use tauri::Emitter;
    let Some(app) = APP.get() else { return };
    let _ = app.emit(
        BACKEND_NOTICE,
        serde_json::json!({
            "kind": kind,
            "source": source,
            "title": title,
            "detail": detail,
            "task_id": task_id,
        }),
    );
}
