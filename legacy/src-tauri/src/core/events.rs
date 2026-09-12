//! Tauri event names for backend→frontend events. This list is the only one:
//! `src/shared/ipc/generated/eventNames.ts` is written from it by `pnpm gen:types`.

/// Declares every event name once, as a Rust const and a row in `ALL`.
macro_rules! events {
    ($($(#[$doc:meta])* $konst:ident => $wire:literal),* $(,)?) => {
        $($(#[$doc])* pub const $konst: &str = $wire;)*
        /// Every event, as (const name, wire name). Read by the generator test.
        #[cfg(test)]
        pub const ALL: &[(&str, &str)] = &[$((stringify!($konst), $wire)),*];
    };
}

events! {
    // ── Workspace / task lifecycle ──────────────────────────────────────────
    WORKSPACE_STUB => "workspace_stub",
    WORKSPACE_READY => "workspace_ready",
    TASK_PAUSED => "task_paused",
    TASK_FINISHED => "task_finished",
    EXPLORER_DISCARDED => "explorer_discarded",

    // ── Confirmation bridge ─────────────────────────────────────────────────
    CONFIRMATION_REQUESTED => "confirmation_requested",
    CONFIRMATION_RESOLVED => "confirmation_resolved",

    // ── Git / worktrees ─────────────────────────────────────────────────────
    WORKTREE_CLOSED => "worktree_closed",
    REBASE_DONE => "rebase_done",
    REBASE_CONFLICT => "rebase_conflict",

    // ── Annotations ─────────────────────────────────────────────────────────
    ANNOTATION_RESOLVED => "annotation_resolved",
    /// An annotation the agent created.
    ANNOTATION_CREATED => "annotation_created",
    /// An annotation body the agent rewrote.
    ANNOTATION_UPDATED => "annotation_updated",

    // ── PTY (agent / terminal) ──────────────────────────────────────────────
    PTY_STARTED => "pty_started",
    PTY_OUTPUT => "pty_output",
    PTY_EXIT => "pty_exit",

    // ── Agent activity (from Claude Code hooks) ─────────────────────────────
    /// One agent changed state: idle / working / waiting on the user.
    AGENT_ACTIVITY => "agent_activity",

    // ── Backend notices ─────────────────────────────────────────────────────
    BACKEND_NOTICE => "backend_notice",
}

use std::sync::OnceLock;

/// The app handle for modules with no `AppHandle`/`State` access. Set once during init.
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

pub fn set_app(handle: tauri::AppHandle) {
    let _ = APP.set(handle);
}

/// How a notice reads in the notification feed.
#[derive(Debug, Clone, Copy, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    Error,
    Attention,
}

/// The `backend_notice` payload.
#[derive(Debug, Clone, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct BackendNotice {
    pub kind: NoticeKind,
    /// Which subsystem is speaking: `git`, `forge`, `agent`.
    pub source: String,
    pub title: String,
    pub detail: Option<String>,
    /// The session it belongs to, when it belongs to one.
    pub task_id: Option<String>,
}

/// Emit a backend notice. No-op before `set_app`.
pub fn notice(
    kind: NoticeKind,
    source: &str,
    title: String,
    detail: Option<String>,
    task_id: Option<&str>,
) {
    use tauri::Emitter;
    let Some(app) = APP.get() else { return };
    let _ = app.emit(
        BACKEND_NOTICE,
        BackendNotice {
            kind,
            source: source.to_string(),
            title,
            detail,
            task_id: task_id.map(str::to_string),
        },
    );
}

#[cfg(test)]
mod tests {
    /// Writes the frontend's runtime name map. `ts-rs` exports types, not values, so
    /// this list is generated the same way the types are: by the test suite.
    #[test]
    fn export_bindings_event_names() {
        let mut out = String::from(
            "// This file was generated from src-tauri/src/core/events.rs. Do not edit it manually.\n\nexport const EVENT = {\n",
        );
        for (konst, wire) in super::ALL {
            out.push_str(&format!("  {konst}: '{wire}',\n"));
        }
        out.push_str(
            "} as const;\n\nexport type EventName = (typeof EVENT)[keyof typeof EVENT];\n",
        );

        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../src/shared/ipc/generated/eventNames.ts");
        if std::fs::read_to_string(&path).ok().as_deref() != Some(out.as_str()) {
            std::fs::write(&path, &out).expect("write eventNames.ts");
        }
    }

    #[test]
    fn every_name_is_unique() {
        let mut wire: Vec<&str> = super::ALL.iter().map(|(_, w)| *w).collect();
        let count = wire.len();
        wire.sort_unstable();
        wire.dedup();
        assert_eq!(wire.len(), count, "two events share a wire name");
    }
}
