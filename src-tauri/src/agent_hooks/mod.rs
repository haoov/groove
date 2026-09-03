//! What each agent is doing, reported by Claude Code's own hooks — not by reading
//! the PTY, whose byte stream is not the text on screen. A tool awaiting approval
//! emits `PreToolUse` then `Notification`, and nothing else until the user answers.
//! Everything here is best-effort.

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
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct AgentActivity {
    /// Task short id — the key, repeated in the payload for the frontend.
    pub task_id: String,
    pub state: AgentState,
    /// The tool in flight, or the one awaiting approval when `state` is Waiting.
    pub tool: Option<ToolCall>,
    /// Claude's closing message for the last finished turn.
    pub last_message: Option<String>,
    /// When this state was entered (unix seconds).
    #[ts(type = "number")]
    pub since: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
#[serde(rename_all = "lowercase")]
pub enum AgentState {
    /// Between turns — free to take a prompt.
    Idle,
    /// Mid-turn: thinking or running a tool.
    Working,
    /// Blocked on the user, in its own terminal.
    Waiting,
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../src/shared/ipc/generated/")]
pub struct ToolCall {
    pub name: String,
    /// One-line rendering of the tool's input, e.g. `cargo test --all`.
    pub detail: Option<String>,
}

pub type ActivityState = Arc<Mutex<HashMap<String, AgentActivity>>>;

pub fn new_state() -> ActivityState {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Snapshot for the frontend's initial hydration.
#[tauri::command]
pub fn get_agent_activity(
    activity: tauri::State<'_, ActivityState>,
) -> Vec<AgentActivity> {
    activity
        .lock()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}

/// Drop an agent's activity when its PTY dies.
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
    /// The task this agent was spawned for, from the hook URL.
    task: Option<String>,
}

/// The subset of the hook payload we use; unknown fields are ignored.
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

/// Must be quick — anything slow here stalls the agent.
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

    let _ = state.app.emit(crate::core::events::AGENT_ACTIVITY, &updated);
    axum::http::StatusCode::OK
}

/// The state machine, driven by event flow — never by message text.
fn apply(entry: &mut AgentActivity, event: &str, payload: &HookPayload) {
    let previous = entry.state;
    match event {
        // A session is ready but between turns.
        "SessionStart" => {
            entry.state = AgentState::Idle;
            entry.tool = None;
        }
        // A prompt was submitted — the turn has started.
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
        // Claude wants the user; the pending tool stays — it is what the question is about.
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

/// One-line rendering of a tool's arguments — identifying fields only, never the payload.
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

    /// PreToolUse then Notification: the pending tool survives into Waiting.
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

    /// An approved tool clears the pending one; Stop ends idle with the closing line.
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

    /// Unknown events leave the state alone.
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
