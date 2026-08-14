use std::{
    collections::HashMap,
    convert::Infallible,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Router,
};
use futures_util::Stream;
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

use crate::confirmation_bridge::Bridge;
use crate::editor_host::State as EditorState;
use crate::task_manager::State as TaskState;

mod tools;

use tools::{dispatch, mcp_tool_definitions};

/// The loopback endpoint, defined ONCE here. Agents (`--mcp-config`) and the
/// status bar derive their URLs from `sse_url`/`endpoint`, so moving the port is
/// a one-line change. Note `127.0.0.1`, not `localhost`: the latter can resolve
/// to `::1`, which this listener does not bind.
pub const HOST: &str = "127.0.0.1";
pub const PORT: u16 = 27413;

const PROTOCOL_VERSION: &str = "2024-11-05";

/// `host:port`, for display.
pub fn endpoint() -> String {
    format!("{HOST}:{PORT}")
}

/// The SSE URL an agent connects to, pinned to the task it was spawned for.
pub fn sse_url(task_id: &str) -> String {
    format!("http://{HOST}:{PORT}/sse?task={task_id}")
}

/// Where an agent's Claude Code hooks POST their payloads, pinned to its task.
pub fn hook_url(task_id: &str) -> String {
    format!("http://{HOST}:{PORT}/hook?task={task_id}")
}

/// Lets the status bar display the real endpoint instead of a stale copy of it.
#[tauri::command]
pub fn get_mcp_endpoint() -> String {
    endpoint()
}

// ─── SSE stream wrapper ───────────────────────────────────────────────────────

/// Wraps the event receiver and unregisters its session entry when the SSE
/// connection drops — otherwise every agent reconnect leaks a dead sender.
struct SseReceiver {
    rx: mpsc::Receiver<Event>,
    sessions: Sessions,
    session_tasks: SessionTasks,
    session_id: String,
}

impl Stream for SseReceiver {
    type Item = Result<Event, Infallible>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx).map(|opt| opt.map(Ok))
    }
}

impl Drop for SseReceiver {
    fn drop(&mut self) {
        if let Ok(mut map) = self.sessions.lock() {
            map.remove(&self.session_id);
        }
        if let Ok(mut map) = self.session_tasks.lock() {
            map.remove(&self.session_id);
        }
    }
}

// ─── Shared session state ─────────────────────────────────────────────────────

type Sessions = Arc<Mutex<HashMap<String, mpsc::Sender<Event>>>>;
/// MCP session id → the task that connection belongs to.
///
/// The global "active task" follows UI focus, which is wrong for an agent: the
/// user switches sessions constantly, and an agent working on task A must keep
/// resolving to A. Each agent is spawned with `--mcp-config` pointing at
/// `/sse?task=<short_id>`, so its connection carries its own task for life.
type SessionTasks = Arc<Mutex<HashMap<String, String>>>;

// ─── Axum shared state ────────────────────────────────────────────────────────

#[derive(Clone)]
struct McpState {
    pool: SqlitePool,
    bridge: Bridge,
    task_state: TaskState,
    editor_state: EditorState,
    sessions: Sessions,
    session_tasks: SessionTasks,
}

impl McpState {
    /// The task this CALLER is working on: its connection's binding when it has
    /// one, else the focused session (a hand-run `claude`, or an older agent).
    fn task_for(&self, mcp_session: &str) -> Option<String> {
        if let Ok(map) = self.session_tasks.lock() {
            if let Some(id) = map.get(mcp_session) {
                return Some(id.clone());
            }
        }
        self.task_state.get_active_task_id()
    }

    /// Re-point a connection (used when an explorer converts into a real task,
    /// which changes the id underneath the agent).
    fn rebind(&self, mcp_session: &str, task_id: &str) {
        if let Ok(mut map) = self.session_tasks.lock() {
            if map.contains_key(mcp_session) {
                map.insert(mcp_session.to_string(), task_id.to_string());
            }
        }
    }
}

// ─── Server entry point ───────────────────────────────────────────────────────

pub async fn start(
    bridge: Bridge,
    pool: SqlitePool,
    task_state: TaskState,
    editor_state: EditorState,
    activity: crate::agent_hooks::ActivityState,
) -> anyhow::Result<()> {
    let app_handle = bridge.app_handle().clone();
    let state = McpState {
        pool,
        bridge,
        task_state,
        editor_state,
        sessions: Arc::new(Mutex::new(HashMap::new())),
        session_tasks: Arc::new(Mutex::new(HashMap::new())),
    };

    // One loopback server for everything agents talk to: MCP tools here, Claude
    // Code hook callbacks in `agent_hooks`.
    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .with_state(state)
        .merge(crate::agent_hooks::router(app_handle, activity));

    let addr: SocketAddr = endpoint().parse()?;
    // A taken port is the one startup failure a user cannot diagnose: every agent
    // tool call then fails with no visible cause. Usually a second Groove instance.
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            crate::events::notice(
                "error",
                "mcp",
                format!("Agent tools are unavailable: {} is busy", endpoint()),
                Some(format!(
                    "{e}. Another Groove instance is probably running — close it and restart. \
                     Agents will start but every tool call will fail."
                )),
                None,
            );
            return Err(e.into());
        }
    };

    tracing::info!("MCP server listening on {}", endpoint());
    axum::serve(listener, app).await?;

    Ok(())
}

// ─── GET /sse — SSE handshake ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct SseQuery {
    /// Short id of the task this client works on (agents pass it; humans don't).
    task: Option<String>,
}

async fn sse_handler(
    Query(q): Query<SseQuery>,
    State(state): State<McpState>,
) -> Sse<SseReceiver> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::channel::<Event>(64);

    if let Some(task) = q.task.filter(|t| !t.is_empty()) {
        tracing::info!("[mcp] session {session_id} bound to task {task}");
        if let Ok(mut map) = state.session_tasks.lock() {
            map.insert(session_id.clone(), task);
        }
    }

    // First event tells the client where to POST messages
    let endpoint_url = format!("/message?sessionId={session_id}");
    let _ = tx.try_send(Event::default().event("endpoint").data(endpoint_url));

    if let Ok(mut map) = state.sessions.lock() {
        map.insert(session_id.clone(), tx);
    }

    Sse::new(SseReceiver {
        rx,
        sessions: state.sessions.clone(),
        session_tasks: state.session_tasks.clone(),
        session_id,
    })
    .keep_alive(KeepAlive::default())
}

// ─── POST /message — JSON-RPC 2.0 receiver ───────────────────────────────────

#[derive(Deserialize)]
struct MessageQuery {
    #[serde(rename = "sessionId")]
    session_id: String,
}

async fn message_handler(
    Query(q): Query<MessageQuery>,
    State(state): State<McpState>,
    body: String,
) -> StatusCode {
    let request: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    // Notifications have no "id" — fire and forget, no response needed
    if request["id"].is_null() && request.get("id").is_none() {
        return StatusCode::ACCEPTED;
    }

    let response = handle_jsonrpc(request, &state, &q.session_id).await;

    let event_data = serde_json::to_string(&response).unwrap_or_default();
    let event = Event::default().event("message").data(event_data);

    let tx = state
        .sessions
        .lock()
        .ok()
        .and_then(|s| s.get(&q.session_id).cloned());

    if let Some(tx) = tx {
        let _ = tx.send(event).await;
    }

    StatusCode::ACCEPTED
}

// ─── JSON-RPC 2.0 dispatch ────────────────────────────────────────────────────

async fn handle_jsonrpc(
    request: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> serde_json::Value {
    let id = request["id"].clone();
    let method = request["method"].as_str().unwrap_or("").to_string();
    let params = request.get("params").cloned().unwrap_or(serde_json::json!({}));

    match method.as_str() {
        "initialize" => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "groove", "version": "1.0" }
            }
        }),

        "tools/list" => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": mcp_tool_definitions() }
        }),

        "tools/call" => {
            let name = params["name"].as_str().unwrap_or("").to_string();
            let args = params["arguments"].clone();
            match dispatch(&name, args, state, mcp_session).await {
                Ok(resp) => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": resp.content,
                        "isError": resp.is_error.unwrap_or(false)
                    }
                }),
                Err(e) => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32603, "message": e.to_string() }
                }),
            }
        }

        _ => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": format!("Method not found: {method}") }
        }),
    }
}
