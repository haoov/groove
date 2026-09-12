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

pub(crate) use tools::mcp_tool_definitions;
use tools::{dispatch, InvalidParams, UnknownTool};

/// Keep `127.0.0.1`, not `localhost`: `localhost` can resolve to `::1`, which is not bound.
pub const HOST: &str = "127.0.0.1";
pub const PORT: u16 = 27413;

const PROTOCOL_VERSION: &str = "2024-11-05";

/// Revisions that define the HTTP+SSE transport this server speaks.
const SUPPORTED_PROTOCOLS: [&str; 2] = [PROTOCOL_VERSION, "2025-03-26"];

// JSON-RPC 2.0 error codes.
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

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

/// The endpoint, for the status bar.
#[tauri::command]
pub fn get_mcp_endpoint() -> String {
    endpoint()
}

// ─── SSE stream wrapper ───────────────────────────────────────────────────────

/// Unregisters its connection entry when the SSE stream drops.
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

/// One live SSE connection and the task it is bound to.
struct Connection {
    tx: mpsc::Sender<Event>,
    task: String,
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
    /// The task this caller is bound to.
    fn task_for(&self, mcp_session: &str) -> Option<String> {
        self.connections
            .lock()
            .ok()
            .and_then(|map| map.get(mcp_session).map(|c| c.task.clone()))
    }

    /// The SSE channel a response goes back on. `None` once the stream drops.
    fn sender_for(&self, mcp_session: &str) -> Option<mpsc::Sender<Event>> {
        self.connections
            .lock()
            .ok()
            .and_then(|map| map.get(mcp_session).map(|c| c.tx.clone()))
    }

    /// Re-point a connection to another task id.
    fn rebind(&self, mcp_session: &str, task_id: &str) {
        if let Ok(mut map) = self.connections.lock() {
            if let Some(conn) = map.get_mut(mcp_session) {
                conn.task = task_id.to_string();
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

    // MCP routes plus the hook routes from `agent_hooks`; every route requires the launch token.
    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .with_state(state)
        .merge(crate::agent_hooks::router(app_handle, activity))
        .layer(axum::middleware::from_fn(auth::require_auth));

    let addr: SocketAddr = endpoint().parse()?;
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            crate::core::events::notice(
                crate::core::events::NoticeKind::Error,
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
    /// The task this client works on. Required.
    task: Option<String>,
}

/// The task a connection is bound to. Missing or empty is refused.
fn bound_task(q: SseQuery) -> Result<String, (StatusCode, &'static str)> {
    q.task.filter(|t| !t.is_empty()).ok_or((
        StatusCode::BAD_REQUEST,
        "an MCP connection needs ?task=<short_id>",
    ))
}

async fn sse_handler(
    Query(q): Query<SseQuery>,
    State(state): State<McpState>,
) -> Result<Sse<SseReceiver>, (StatusCode, &'static str)> {
    let task = bound_task(q)?;

    // Bind only a session that exists; an unknown id would answer every tool with "no task".
    let known = crate::core::db::store::sessions::get_opt(&state.pool, &task)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "cannot read the session store",
            )
        })?
        .is_some();
    if !known {
        return Err((StatusCode::NOT_FOUND, "?task= names no open session"));
    }

    let session_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::channel::<Event>(64);
    tracing::info!("[mcp] session {session_id} bound to task {task}");

    // First event tells the client where to POST messages
    let endpoint_url = format!("/message?sessionId={session_id}");
    let _ = tx.try_send(Event::default().event("endpoint").data(endpoint_url));

    if let Ok(mut map) = state.connections.lock() {
        map.insert(session_id.clone(), Connection { tx, task });
    }

    Ok(Sse::new(SseReceiver {
        rx,
        connections: state.connections.clone(),
        session_id,
    })
    .keep_alive(KeepAlive::default()))
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
    // No live stream to answer on.
    if state.sender_for(&q.session_id).is_none() {
        return StatusCode::NOT_FOUND;
    }

    let request: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    // Notifications carry no id and get no response.
    if request.get("id").is_none_or(|v| v.is_null()) {
        return StatusCode::ACCEPTED;
    }

    // The answer travels on the SSE stream, so a call blocked on the user holds no request.
    let state = state.clone();
    let mcp_session = q.session_id.clone();
    tokio::spawn(async move {
        let response = handle_jsonrpc(request, &state, &mcp_session).await;
        let data = serde_json::to_string(&response).unwrap_or_default();
        // Looked up now, not before: the stream may have dropped while the call ran.
        if let Some(tx) = state.sender_for(&mcp_session) {
            let _ = tx.send(Event::default().event("message").data(data)).await;
        }
    });

    StatusCode::ACCEPTED
}

/// Progress interval for a blocked call. Keep it well under the client's 5-minute SSE idle timeout.
const PROGRESS_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// Send progress notifications while a tool call is blocked on the user.
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

fn result(id: &serde_json::Value, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: &serde_json::Value, code: i64, message: String) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

/// The revision to answer `initialize` with: the client's when we speak it.
fn negotiated_protocol(params: &serde_json::Value) -> &str {
    let asked = params["protocolVersion"].as_str().unwrap_or_default();
    SUPPORTED_PROTOCOLS
        .into_iter()
        .find(|v| *v == asked)
        .unwrap_or(PROTOCOL_VERSION)
}

/// A tool-call failure's JSON-RPC code. Bad arguments are the caller's, not ours.
fn tool_error_code(e: &anyhow::Error) -> i64 {
    match e.downcast_ref::<InvalidParams>().is_some() || e.downcast_ref::<UnknownTool>().is_some() {
        true => INVALID_PARAMS,
        false => INTERNAL_ERROR,
    }
}

/// A malformed envelope: `jsonrpc` is not "2.0", or `method` is not a string.
fn envelope_error(request: &serde_json::Value) -> Option<&'static str> {
    if request["jsonrpc"].as_str() != Some("2.0") {
        return Some("jsonrpc must be the string \"2.0\"");
    }
    if !request["method"].is_string() {
        return Some("method must be a string");
    }
    None
}

async fn handle_jsonrpc(
    request: serde_json::Value,
    state: &McpState,
    mcp_session: &str,
) -> serde_json::Value {
    let id = request["id"].clone();
    if let Some(why) = envelope_error(&request) {
        return error(&id, INVALID_REQUEST, why.to_string());
    }
    let method = request["method"].as_str().unwrap_or("").to_string();
    let params = request
        .get("params")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    match method.as_str() {
        "initialize" => result(
            &id,
            serde_json::json!({
                "protocolVersion": negotiated_protocol(&params),
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "groove", "version": "1.0" }
            }),
        ),

        "ping" => result(&id, serde_json::json!({})),

        "tools/list" => result(&id, serde_json::json!({ "tools": mcp_tool_definitions() })),

        "tools/call" => {
            let Some(name) = params["name"].as_str().map(str::to_string) else {
                return error(
                    &id,
                    INVALID_PARAMS,
                    "tools/call needs a string name".to_string(),
                );
            };
            let args = params["arguments"].clone();
            let token = params["_meta"]["progressToken"].clone();
            let beat = (!token.is_null()).then(|| spawn_progress(state, mcp_session, token));
            let outcome = dispatch(&name, args, state, mcp_session).await;
            if let Some(beat) = beat {
                beat.abort();
            }
            match outcome {
                Ok(resp) => result(
                    &id,
                    serde_json::json!({
                        "content": resp.content,
                        "isError": resp.is_error.unwrap_or(false)
                    }),
                ),
                Err(e) => error(&id, tool_error_code(&e), format!("{name}: {e}")),
            }
        }

        _ => error(&id, METHOD_NOT_FOUND, format!("Method not found: {method}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bound_task, envelope_error, negotiated_protocol, tool_error_code, InvalidParams, SseQuery,
        UnknownTool, INTERNAL_ERROR, INVALID_PARAMS, PROTOCOL_VERSION,
    };
    use axum::http::StatusCode;

    #[test]
    fn a_broken_envelope_is_an_invalid_request() {
        let good = serde_json::json!({ "jsonrpc": "2.0", "method": "ping", "id": 1 });
        assert!(envelope_error(&good).is_none());

        for bad in [
            serde_json::json!({ "method": "ping", "id": 1 }),
            serde_json::json!({ "jsonrpc": "1.0", "method": "ping", "id": 1 }),
            serde_json::json!({ "jsonrpc": 2.0, "method": "ping", "id": 1 }),
        ] {
            assert_eq!(
                envelope_error(&bad),
                Some("jsonrpc must be the string \"2.0\"")
            );
        }

        let no_method = serde_json::json!({ "jsonrpc": "2.0", "id": 1 });
        assert_eq!(envelope_error(&no_method), Some("method must be a string"));
        let numeric = serde_json::json!({ "jsonrpc": "2.0", "method": 7, "id": 1 });
        assert_eq!(envelope_error(&numeric), Some("method must be a string"));
    }

    #[test]
    fn a_supported_protocol_is_echoed_back() {
        let asked = serde_json::json!({ "protocolVersion": "2025-03-26" });
        assert_eq!(negotiated_protocol(&asked), "2025-03-26");

        let unknown = serde_json::json!({ "protocolVersion": "1999-01-01" });
        assert_eq!(negotiated_protocol(&unknown), PROTOCOL_VERSION);
        assert_eq!(
            negotiated_protocol(&serde_json::json!({})),
            PROTOCOL_VERSION
        );
    }

    #[test]
    fn bad_arguments_are_the_callers_fault() {
        let params = anyhow::Error::new(InvalidParams("missing field `worktree_id`".into()));
        assert_eq!(tool_error_code(&params), INVALID_PARAMS);
        let unknown = anyhow::Error::new(UnknownTool("get_the_moon".into()));
        assert_eq!(tool_error_code(&unknown), INVALID_PARAMS);
        assert_eq!(
            tool_error_code(&anyhow::anyhow!("the forge said no")),
            INTERNAL_ERROR
        );
    }

    #[test]
    fn a_connection_that_names_no_task_is_refused() {
        assert_eq!(
            bound_task(SseQuery { task: None }).unwrap_err().0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            bound_task(SseQuery {
                task: Some(String::new())
            })
            .unwrap_err()
            .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            bound_task(SseQuery {
                task: Some("gh-groove-3".into())
            })
            .unwrap(),
            "gh-groove-3"
        );
    }
}
