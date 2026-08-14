//! What each agent is doing, reported by Claude Code's own hooks.
//!
//! Agents run in a PTY, so the obvious way to know their state is to read the
//! terminal — but Claude's TUI positions text with cursor moves and redraws in
//! place, so the byte stream is not the text you see, and any parser we wrote
//! would be coupled to a layout that isn't ours to depend on.
//!
//! Instead, `agent_manager` spawns every agent with inline `--settings` hooks
//! that POST here. Hooks are a supported contract, the payloads name the tool
//! directly, and they arrive a few times a minute rather than thousands of times
//! a second. Verified against Claude Code 2.1.220: a tool awaiting approval emits
//! `PreToolUse` and then `Notification`, and nothing else until the user answers.
//!
//! Everything here is best-effort. A hook that fails to reach us costs a status
//! update and nothing more — never the agent's progress.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Query, State},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

/// What an agent is doing right now, per task.
#[derive(Debug, Clone, Serialize)]
pub struct AgentActivity {
    /// Task short id — the key, repeated in the payload for the frontend.
    pub task_id: String,
    pub state: AgentState,
    /// The tool in flight, or the one awaiting approval when `state` is Waiting.
    pub tool: Option<ToolCall>,
    /// Claude's closing message for the last finished turn.
    pub last_message: Option<String>,
    /// When this state was entered (unix seconds).
    pub since: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentState {
    /// Between turns — free to take a prompt.
    Idle,
    /// Mid-turn: thinking or running a tool.
    Working,
    /// Blocked on the user, in its own terminal.
    Waiting,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCall {
    pub name: String,
    /// One-line rendering of the tool's input, e.g. `cargo test --all`.
    pub detail: Option<String>,
}

pub type ActivityState = Arc<Mutex<HashMap<String, AgentActivity>>>;

pub fn new_state() -> ActivityState {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Snapshot for the frontend's initial hydration (state is in-memory, so a
/// restart legitimately knows nothing until the next hook arrives).
#[tauri::command]
pub fn get_agent_activity(
    activity: tauri::State<'_, ActivityState>,
) -> Vec<AgentActivity> {
    activity
        .lock()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}

/// Drop an agent's activity when its PTY dies, so a stale "waiting" can't
/// outlive the process it described.
pub fn forget(activity: &ActivityState, task_id: &str) {
    if let Ok(mut map) = activity.lock() {
        map.remove(task_id);
    }
}

// ─── HTTP surface ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct HookState {
    app: AppHandle,
    activity: ActivityState,
}

#[derive(Deserialize)]
struct HookQuery {
    /// The task this agent was spawned for. Passed by us in the hook URL rather
    /// than derived from `session_id`: an agent resuming a legacy conversation
    /// runs under an id we didn't generate, so only the URL is authoritative.
    task: Option<String>,
}

/// The subset of the hook payload we use. Unknown fields are ignored, so new
/// Claude Code versions can add to it freely.
#[derive(Deserialize)]
struct HookPayload {
    hook_event_name: Option<String>,
    tool_name: Option<String>,
    tool_input: Option<serde_json::Value>,
    last_assistant_message: Option<String>,
}

pub fn router(app: AppHandle, activity: ActivityState) -> Router {
    Router::new()
        .route("/hook", post(hook_handler))
        .with_state(HookState { app, activity })
}

/// Hooks expect a quick, boring reply — anything slow here stalls the agent, so
/// this only touches an in-memory map and emits an event.
async fn hook_handler(
    Query(q): Query<HookQuery>,
    State(state): State<HookState>,
    Json(payload): Json<HookPayload>,
) -> axum::http::StatusCode {
    let Some(task_id) = q.task.filter(|t| !t.is_empty()) else {
        return axum::http::StatusCode::OK;
    };
    let Some(event) = payload.hook_event_name.as_deref() else {
        return axum::http::StatusCode::OK;
    };

    let updated = {
        let Ok(mut map) = state.activity.lock() else {
            return axum::http::StatusCode::OK;
        };
        let entry = map.entry(task_id.clone()).or_insert_with(|| AgentActivity {
            task_id: task_id.clone(),
            state: AgentState::Idle,
            tool: None,
            last_message: None,
            since: now(),
        });
        apply(entry, event, &payload);
        entry.clone()
    };

    let _ = state.app.emit(crate::events::AGENT_ACTIVITY, &updated);
    axum::http::StatusCode::OK
}

/// The state machine. Derived from the event FLOW, never from message text:
/// `Notification`'s message is a generic "Claude needs your permission", and its
/// wording is Claude's to change.
fn apply(entry: &mut AgentActivity, event: &str, payload: &HookPayload) {
    let previous = entry.state;
    match event {
        // A session is ready but between turns.
        "SessionStart" => {
            entry.state = AgentState::Idle;
            entry.tool = None;
        }
        // Someone (the user, or the pill) submitted a prompt. Without this the
        // status would lag until the agent's first tool call.
        "UserPromptSubmit" => {
            entry.state = AgentState::Working;
            entry.tool = None;
            entry.last_message = None;
        }
        "PreToolUse" => {
            entry.state = AgentState::Working;
            entry.tool = payload.tool_name.as_ref().map(|name| ToolCall {
                name: name.clone(),
                detail: summarize_tool_input(name, payload.tool_input.as_ref()),
            });
        }
        // The tool ran, so whatever it was is no longer pending approval.
        "PostToolUse" => {
            entry.state = AgentState::Working;
            entry.tool = None;
        }
        // Claude wants the user. The pending `PreToolUse` tool (if any) is what
        // it is asking about, so it deliberately stays on the entry.
        "Notification" => entry.state = AgentState::Waiting,
        "Stop" => {
            entry.state = AgentState::Idle;
            entry.tool = None;
            entry.last_message = payload
                .last_assistant_message
                .as_ref()
                .map(|m| first_line(m));
        }
        _ => {}
    }
    if entry.state != previous {
        entry.since = now();
    }
}

/// A one-line, human-readable rendering of the tool's arguments. Only the fields
/// worth showing in a status line — never the whole payload (file contents and
/// prompts belong in the terminal, not in a pill).
fn summarize_tool_input(name: &str, input: Option<&serde_json::Value>) -> Option<String> {
    let input = input?;
    let field = |key: &str| input.get(key).and_then(|v| v.as_str());
    let detail = match name {
        "Bash" | "BashOutput" => field("command"),
        "Read" | "Write" | "Edit" | "MultiEdit" | "NotebookEdit" => field("file_path"),
        "Glob" | "Grep" => field("pattern"),
        "WebFetch" => field("url"),
        "WebSearch" => field("query"),
        "Task" => field("description"),
        // MCP tools and anything new: show the most identifying field present.
        _ => field("task_id")
            .or_else(|| field("file_path"))
            .or_else(|| field("title")),
    }?;
    Some(shorten(detail, 80))
}

fn first_line(s: &str) -> String {
    let line = s.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
    shorten(line, 140)
}

/// Truncate on a char boundary (paths and messages are not always ASCII).
fn shorten(s: &str, max: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        return flat;
    }
    let cut: String = flat.chars().take(max.saturating_sub(1)).collect();
    format!("{cut}…")
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> AgentActivity {
        AgentActivity {
            task_id: "TASKS2-1".into(),
            state: AgentState::Idle,
            tool: None,
            last_message: None,
            since: 0,
        }
    }

    fn payload(tool: Option<&str>, input: Option<serde_json::Value>) -> HookPayload {
        HookPayload {
            hook_event_name: None,
            tool_name: tool.map(str::to_string),
            tool_input: input,
            last_assistant_message: None,
        }
    }

    /// The sequence observed from Claude Code 2.1.220 when a tool needs approval:
    /// PreToolUse then Notification, and nothing more until the user answers. The
    /// pending tool must survive into Waiting — it's what the question is about.
    #[test]
    fn permission_prompt_keeps_the_pending_tool() {
        let mut e = entry();
        apply(&mut e, "UserPromptSubmit", &payload(None, None));
        assert_eq!(e.state, AgentState::Working);

        apply(
            &mut e,
            "PreToolUse",
            &payload(Some("Write"), Some(serde_json::json!({ "file_path": "a/b.rs" }))),
        );
        apply(&mut e, "Notification", &payload(None, None));

        assert_eq!(e.state, AgentState::Waiting);
        let tool = e.tool.as_ref().expect("the awaited tool");
        assert_eq!(tool.name, "Write");
        assert_eq!(tool.detail.as_deref(), Some("a/b.rs"));
    }

    /// An approved tool runs, so nothing is pending; the turn then ends idle with
    /// Claude's closing line.
    #[test]
    fn approved_tool_then_turn_end() {
        let mut e = entry();
        apply(&mut e, "PreToolUse", &payload(Some("Bash"), Some(serde_json::json!({ "command": "cargo test" }))));
        apply(&mut e, "PostToolUse", &payload(Some("Bash"), None));
        assert_eq!(e.state, AgentState::Working);
        assert!(e.tool.is_none());

        let mut stop = payload(None, None);
        stop.last_assistant_message = Some("Fixed the parser.\n\nDetails below.".into());
        apply(&mut e, "Stop", &stop);
        assert_eq!(e.state, AgentState::Idle);
        assert_eq!(e.last_message.as_deref(), Some("Fixed the parser."));
    }

    /// Unknown events must not disturb the state — new Claude Code versions are
    /// free to add hooks we don't handle.
    #[test]
    fn unknown_event_is_inert() {
        let mut e = entry();
        apply(&mut e, "PreToolUse", &payload(Some("Read"), Some(serde_json::json!({ "file_path": "x" }))));
        let before = e.state;
        apply(&mut e, "PreCompact", &payload(None, None));
        assert_eq!(e.state, before);
        assert!(e.tool.is_some());
    }

    #[test]
    fn tool_detail_is_bounded_and_char_safe() {
        let long = "é".repeat(400);
        let d = summarize_tool_input("Bash", Some(&serde_json::json!({ "command": long })))
            .expect("a detail");
        assert!(d.chars().count() <= 80, "{} chars", d.chars().count());
    }
}
