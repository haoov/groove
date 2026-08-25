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

use crate::approvals::Bridge;
use crate::editor_host::State as EditorState;
use crate::task_manager::State as TaskState;

pub(crate) mod auth;
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

/// Wraps the event receiver and unregisters its connection entry when the SSE
/// stream drops — otherwise every agent reconnect leaks a dead sender.
struct SseReceiver {
    rx: mpsc::Receiver<Event>,
    connections: Connections,
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
        if let Ok(mut map) = self.connections.lock() {
            map.remove(&self.session_id);
        }
    }
}

// ─── Shared connection state ──────────────────────────────────────────────────

/// One live SSE connection: where its responses go, and the task it belongs to.
///
/// The global "active task" follows UI focus, which is wrong for an agent: the
/// user switches sessions constantly, and an agent working on task A must keep
/// resolving to A. Each agent is spawned with an `--mcp-config` pointing at
/// `/sse?task=<short_id>`, so its connection carries its own task for life.
struct Connection {
    tx: mpsc::Sender<Event>,
    task: Option<String>,
}

type Connections = Arc<Mutex<HashMap<String, Connection>>>;

// ─── Axum shared state ────────────────────────────────────────────────────────

#[derive(Clone)]
struct McpState {
    pool: SqlitePool,
    bridge: Bridge,
    task_state: TaskState,
    editor_state: EditorState,
    connections: Connections,
}

impl McpState {
    /// The task this CALLER is working on: its connection's binding when it has
    /// one, else the focused session (a hand-run `claude`, or an older agent).
    fn task_for(&self, mcp_session: &str) -> Option<String> {
        if let Ok(map) = self.connections.lock() {
            if let Some(task) = map.get(mcp_session).and_then(|c| c.task.clone()) {
                return Some(task);
            }
        }
        self.task_state.get_active_task_id()
    }

    /// Re-point a connection (used when an explorer converts into a real task,
    /// which changes the id underneath the agent).
    fn rebind(&self, mcp_session: &str, task_id: &str) {
        if let Ok(mut map) = self.connections.lock() {
            if let Some(conn) = map.get_mut(mcp_session) {
                conn.task = Some(task_id.to_string());
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
        connections: Arc::new(Mutex::new(HashMap::new())),
    };

    // One loopback server for everything agents talk to: MCP tools here, Claude
    // Code hook callbacks in `agent_hooks`. Every route requires the launch
    // token — the port is open to any local process otherwise.
    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .with_state(state)
        .merge(crate::agent_hooks::router(app_handle, activity))
        .layer(axum::middleware::from_fn(auth::require_auth));

    let addr: SocketAddr = endpoint().parse()?;
    // A taken port is the one startup failure a user cannot diagnose: every agent
    // tool call then fails with no visible cause. Usually a second Groove instance.
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            crate::core::events::notice(
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

    let task = q.task.filter(|t| !t.is_empty());
    if let Some(task) = &task {
        tracing::info!("[mcp] session {session_id} bound to task {task}");
    }

    // First event tells the client where to POST messages
    let endpoint_url = format!("/message?sessionId={session_id}");
    let _ = tx.try_send(Event::default().event("endpoint").data(endpoint_url));

    if let Ok(mut map) = state.connections.lock() {
        map.insert(session_id.clone(), Connection { tx, task });
    }

    Sse::new(SseReceiver {
        rx,
        connections: state.connections.clone(),
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

    // Notifications carry no id (a JSON-RPC request must not use a null one
    // either) — fire and forget, no response needed.
    if request.get("id").is_none_or(|v| v.is_null()) {
        return StatusCode::ACCEPTED;
    }

    let response = handle_jsonrpc(request, &state, &q.session_id).await;

    let event_data = serde_json::to_string(&response).unwrap_or_default();
    let event = Event::default().event("message").data(event_data);

    let tx = state
        .connections
        .lock()
        .ok()
        .and_then(|s| s.get(&q.session_id).map(|c| c.tx.clone()));

    if let Some(tx) = tx {
        let _ = tx.send(event).await;
    }

    StatusCode::ACCEPTED
}

/// How often a blocked call reports that it is still blocked. The client's idle
/// timeout is 5 minutes on SSE, so this leaves several missed ticks of margin.
const PROGRESS_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// Keep a blocked tool call alive on the client side.
///
/// A gated write waits for a human to approve it, and `post_and_wait` has no
/// deadline of its own — but Claude Code aborts a call that has gone silent for
/// its idle timeout, and the retry that follows queues the request a second time.
/// Progress notifications reset that idle timer (they do NOT extend the overall
/// wall clock), which is exactly the one a human-paced approval trips.
///
/// Only sent when the caller supplied a progressToken, per the MCP spec.
fn spawn_progress(
    state: &McpState,
    mcp_session: &str,
    token: serde_json::Value,
) -> tokio::task::JoinHandle<()> {
    let tx = state
        .connections
        .lock()
        .ok()
        .and_then(|m| m.get(mcp_session).map(|c| c.tx.clone()));
    tokio::spawn(async move {
        let Some(tx) = tx else { return };
        let mut ticks: u64 = 0;
        loop {
            tokio::time::sleep(PROGRESS_EVERY).await;
            ticks += 1;
            let note = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/progress",
                "params": {
                    "progressToken": token,
                    "progress": ticks,
                    "message": "still waiting for the user to decide in Groove",
                }
            });
            // A closed stream means the agent is gone; stop rather than spin.
            if tx
                .send(Event::default().event("message").data(note.to_string()))
                .await
                .is_err()
            {
                return;
            }
        }
    })
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
            // A gated write can sit on a human for minutes; say we are alive.
            let token = params["_meta"]["progressToken"].clone();
            let beat = (!token.is_null()).then(|| spawn_progress(state, mcp_session, token));
            let outcome = dispatch(&name, args, state, mcp_session).await;
            if let Some(beat) = beat {
                beat.abort();
            }
            match outcome {
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
