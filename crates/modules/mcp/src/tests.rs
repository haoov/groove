use std::time::Duration;

use groove_tools as tools;
use serde_json::{Value, json};
use tokio::runtime::Runtime;

use crate::{Server, serve};

const SESSION: &str = "gh-haoov-groove-50";

fn started() -> (Runtime, Server) {
    answering(|call| {
        call.reply
            .said(format!("{} on {}", call.tool, call.session))
    })
}

/// A server whose calls the given closure answers, as the app would.
fn answering(on_call: impl Fn(crate::Call) + Send + Sync + 'static) -> (Runtime, Server) {
    let runtime = Runtime::new().expect("a runtime");
    let server = serve(runtime.handle(), on_call).expect("a loopback port");
    (runtime, server)
}

/// A stream on the server, and the path its first event says to post to.
async fn connected(server: &Server) -> (reqwest::Response, String) {
    let mut stream = reqwest::Client::new()
        .get(server.sse_url(SESSION))
        .bearer_auth(&server.token)
        .send()
        .await
        .expect("the stream opens");
    assert_eq!(stream.status(), 200);
    let first = one_event(&mut stream).await;
    assert!(first.starts_with("event: endpoint"), "{first}");
    (stream, data_of(&first))
}

/// Up to the blank line that ends one event, and no longer than the patience for it.
async fn one_event(stream: &mut reqwest::Response) -> String {
    let read = async {
        let mut text = String::new();
        while let Some(chunk) = stream.chunk().await.expect("the stream holds") {
            text.push_str(&String::from_utf8_lossy(&chunk));
            if let Some(end) = text.find("\n\n") {
                return text[..end].to_string();
            }
        }
        panic!("the stream ended with nothing on it");
    };
    tokio::time::timeout(Duration::from_secs(5), read)
        .await
        .expect("an event within five seconds")
}

fn data_of(event: &str) -> String {
    event
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .expect("a data line")
        .to_string()
}

async fn ask(server: &Server, post: &str, call: Value) -> reqwest::StatusCode {
    reqwest::Client::new()
        .post(format!("http://127.0.0.1:{}{post}", server.port))
        .bearer_auth(&server.token)
        .json(&call)
        .send()
        .await
        .expect("the server takes it")
        .status()
}

async fn answer(stream: &mut reqwest::Response) -> Value {
    let event = one_event(stream).await;
    assert!(event.starts_with("event: message"), "{event}");
    serde_json::from_str(&data_of(&event)).expect("json")
}

fn call(id: u64, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

#[test]
fn the_first_event_says_where_to_post() {
    let (runtime, server) = started();
    runtime.block_on(async {
        let (_stream, post) = connected(&server).await;
        assert!(post.starts_with("/message?sessionId="), "{post}");
    });
}

#[test]
fn a_stream_without_the_token_is_refused() {
    let (runtime, server) = started();
    let status = runtime.block_on(async {
        reqwest::Client::new()
            .get(server.sse_url(SESSION))
            .bearer_auth("not-the-token")
            .send()
            .await
            .expect("the server answers")
            .status()
    });
    assert_eq!(status, 401);
}

#[test]
fn a_stream_bound_to_no_session_is_refused() {
    let (runtime, server) = started();
    let status = runtime.block_on(async {
        reqwest::Client::new()
            .get(format!("http://127.0.0.1:{}/sse", server.port))
            .bearer_auth(&server.token)
            .send()
            .await
            .expect("the server answers")
            .status()
    });
    assert_eq!(status, 400);
}

#[test]
fn initialize_answers_in_the_revision_the_client_asked_for() {
    let (runtime, server) = started();
    runtime.block_on(async {
        let (mut stream, post) = connected(&server).await;
        let asked = json!({ "protocolVersion": "2025-03-26" });
        assert_eq!(ask(&server, &post, call(1, "initialize", asked)).await, 202);
        let answer = answer(&mut stream).await;
        assert_eq!(answer["id"], 1);
        assert_eq!(answer["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(answer["result"]["serverInfo"]["name"], "groove");
    });
}

#[test]
fn initialize_answers_in_our_own_revision_for_one_we_do_not_speak() {
    let (runtime, server) = started();
    runtime.block_on(async {
        let (mut stream, post) = connected(&server).await;
        let asked = json!({ "protocolVersion": "1999-01-01" });
        ask(&server, &post, call(1, "initialize", asked)).await;
        let answer = answer(&mut stream).await;
        assert_eq!(answer["result"]["protocolVersion"], "2024-11-05");
    });
}

#[test]
fn tools_list_answers_for_every_tool_there_is() {
    let (runtime, server) = started();
    runtime.block_on(async {
        let (mut stream, post) = connected(&server).await;
        ask(&server, &post, call(2, "tools/list", json!({}))).await;
        let answer = answer(&mut stream).await;
        let listed = answer["result"]["tools"].as_array().expect("the tools");
        assert_eq!(listed.len(), tools::all().len());
        let names: Vec<&str> = listed
            .iter()
            .filter_map(|one| one["name"].as_str())
            .collect();
        assert!(names.contains(&"get_active_task"), "{names:?}");
        assert!(names.contains(&"git_commit"), "{names:?}");
        assert!(listed[0]["inputSchema"]["type"] == "object");
    });
}

#[test]
fn a_notification_is_taken_and_never_answered() {
    let (runtime, server) = started();
    runtime.block_on(async {
        let (mut stream, post) = connected(&server).await;
        let told = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert_eq!(ask(&server, &post, told).await, 202);
        ask(&server, &post, call(7, "ping", json!({}))).await;
        let answer = answer(&mut stream).await;
        assert_eq!(answer["id"], 7, "the ping's answer came first");
    });
}

#[test]
fn a_message_on_a_stream_that_is_gone_is_not_found() {
    let (runtime, server) = started();
    let status = runtime.block_on(async {
        let post = "/message?sessionId=nothing";
        ask(&server, post, call(1, "ping", json!({}))).await
    });
    assert_eq!(status, 404);
}

#[test]
fn a_call_that_is_not_jsonrpc_is_refused() {
    let (runtime, server) = started();
    runtime.block_on(async {
        let (mut stream, post) = connected(&server).await;
        let bad = json!({ "id": 3, "method": "ping" });
        ask(&server, &post, bad).await;
        assert_eq!(answer(&mut stream).await["error"]["code"], -32600);
    });
}

#[test]
fn a_method_we_do_not_answer_says_so() {
    let (runtime, server) = started();
    runtime.block_on(async {
        let (mut stream, post) = connected(&server).await;
        ask(&server, &post, call(4, "resources/list", json!({}))).await;
        assert_eq!(answer(&mut stream).await["error"]["code"], -32601);
    });
}

#[test]
fn a_call_reaches_the_app_with_its_session_and_its_arguments() {
    let (runtime, server) = answering(|call| {
        let said = json!({
            "session": call.session,
            "tool": call.tool,
            "worktree": call.text("worktree_id"),
        });
        call.reply.json(&said);
    });
    runtime.block_on(async {
        let (mut stream, post) = connected(&server).await;
        let asked = json!({
            "name": "get_mr_state",
            "arguments": { "worktree_id": "w-1" },
        });
        assert_eq!(ask(&server, &post, call(9, "tools/call", asked)).await, 202);
        let answer = answer(&mut stream).await;
        assert_eq!(answer["result"]["isError"], false);
        let said: Value = serde_json::from_str(
            answer["result"]["content"][0]["text"]
                .as_str()
                .expect("the text"),
        )
        .expect("json");
        assert_eq!(said["session"], SESSION);
        assert_eq!(said["tool"], "get_mr_state");
        assert_eq!(said["worktree"], "w-1");
    });
}

#[test]
fn a_tool_that_failed_says_so_in_its_answer() {
    let (runtime, server) = answering(|call| call.reply.failed("no worktree by that id"));
    runtime.block_on(async {
        let (mut stream, post) = connected(&server).await;
        let asked = json!({ "name": "get_mr_state", "arguments": {} });
        ask(&server, &post, call(1, "tools/call", asked)).await;
        let answer = answer(&mut stream).await;
        assert_eq!(answer["result"]["isError"], true);
        assert_eq!(
            answer["result"]["content"][0]["text"],
            "no worktree by that id"
        );
    });
}

#[test]
fn a_tool_groove_does_not_have_is_refused_before_the_app_sees_it() {
    let (runtime, server) = answering(|call| call.reply.said("this should never run"));
    runtime.block_on(async {
        let (mut stream, post) = connected(&server).await;
        let asked = json!({ "name": "rm_rf", "arguments": {} });
        ask(&server, &post, call(1, "tools/call", asked)).await;
        let answer = answer(&mut stream).await;
        assert_eq!(answer["error"]["code"], -32602);
    });
}

#[test]
fn an_app_that_never_answers_fails_the_call() {
    let (runtime, server) = answering(|call| drop(call.reply));
    runtime.block_on(async {
        let (mut stream, post) = connected(&server).await;
        let asked = json!({ "name": "get_active_task", "arguments": {} });
        ask(&server, &post, call(1, "tools/call", asked)).await;
        let answer = answer(&mut stream).await;
        assert_eq!(answer["error"]["code"], -32603);
    });
}
