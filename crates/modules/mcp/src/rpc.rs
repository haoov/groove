//! JSON-RPC 2.0: what the agent asks, and what goes back on its stream.

use groove_tools as tools;
use serde_json::{Value, json};

/// The revision this server speaks, and the later one it also answers.
const PROTOCOL: &str = "2024-11-05";
const ALSO: &str = "2025-03-26";

const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INTERNAL_ERROR: i64 = -32603;

/// One call of a session, answered.
pub async fn answer(call: &Value, _session: &str) -> Value {
    let id = call["id"].clone();
    if let Some(why) = envelope(call) {
        return error(&id, INVALID_REQUEST, why);
    }
    match call["method"].as_str().unwrap_or_default() {
        "initialize" => result(&id, hello(&call["params"])),
        "ping" => result(&id, json!({})),
        "tools/list" => result(&id, json!({ "tools": listed() })),
        "tools/call" => error(&id, INTERNAL_ERROR, "groove answers no call yet"),
        other => error(&id, METHOD_NOT_FOUND, &format!("no method {other}")),
    }
}

/// What a malformed envelope is: the wrong version, or no method to call.
fn envelope(call: &Value) -> Option<&'static str> {
    if call["jsonrpc"].as_str() != Some("2.0") {
        return Some("jsonrpc must be the string \"2.0\"");
    }
    if !call["method"].is_string() {
        return Some("method must be a string");
    }
    None
}

/// What the server says it is, in the revision the client asked for.
fn hello(params: &Value) -> Value {
    let asked = params["protocolVersion"].as_str().unwrap_or_default();
    let spoken = match asked == ALSO {
        true => ALSO,
        false => PROTOCOL,
    };
    json!({
        "protocolVersion": spoken,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "groove", "version": env!("CARGO_PKG_VERSION") },
    })
}

fn listed() -> Vec<Value> {
    tools::all().iter().map(tools::Tool::listed).collect()
}

fn result(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}
