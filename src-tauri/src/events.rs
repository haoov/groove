//! Tauri event names — the single source of truth for backend→frontend events.
//!
//! This is a hand-mirrored contract with the frontend: every name here must match
//! `src/lib/events.ts`. Keep the two files in sync.

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
pub const FILE_CHANGED: &str = "file_changed";
pub const REBASE_DONE: &str = "rebase_done";
pub const REBASE_CONFLICT: &str = "rebase_conflict";

// ── Annotations ─────────────────────────────────────────────────────────────
pub const ANNOTATION_RESOLVED: &str = "annotation_resolved";
/// An annotation the AGENT created — the UI adds its own optimistically, but
/// without this event agent notes only showed up after reopening the session.
pub const ANNOTATION_CREATED: &str = "annotation_created";

// ── PTY (agent / terminal) ──────────────────────────────────────────────────
pub const PTY_STARTED: &str = "pty_started";
pub const PTY_OUTPUT: &str = "pty_output";
pub const PTY_EXIT: &str = "pty_exit";

// ── Agent activity (from Claude Code hooks) ─────────────────────────────────
/// One agent changed state: idle / working / waiting on the user.
pub const AGENT_ACTIVITY: &str = "agent_activity";

// ── Backend notices ─────────────────────────────────────────────────────────
/// An operational warning from work the user didn't directly trigger — payload
/// mirrors the frontend's `NotificationInput`, so it lands in the feed as-is.
pub const BACKEND_NOTICE: &str = "backend_notice";

use std::sync::OnceLock;

/// The app handle, for modules with no `AppHandle`/`State` access.
///
/// Same pattern as `core::config`: git provisioning runs deep
/// inside call chains that never needed a handle, and silently swallowing a failed
/// fetch is worse than a global. Set once during init.
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

pub fn set_app(handle: tauri::AppHandle) {
    let _ = APP.set(handle);
}

/// Tell the user something went sideways in background work.
///
/// Best-effort: before `set_app` (or in tests) this is a no-op, because a missing
/// notification must never fail the operation that tried to report it.
pub fn notice(kind: &str, source: &str, title: String, detail: Option<String>, task_id: Option<&str>) {
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
