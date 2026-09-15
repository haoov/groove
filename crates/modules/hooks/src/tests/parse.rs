use groove_types::{HookKind, ToolCall};

use crate::parse::post;

#[test]
fn a_payload_carries_the_event_and_the_tool() {
    let body = br#"{
        "hook_event_name": "PreToolUse",
        "session_id": "claude-uuid",
        "tool_name": "Bash",
        "tool_input": { "command": "cargo test --all", "description": "run the tests" }
    }"#;
    let post = post("gh-groove-50", body).expect("a hook");
    assert_eq!(post.session.as_str(), "gh-groove-50");
    assert_eq!(post.kind, HookKind::PreToolUse);
    assert_eq!(
        post.tool,
        Some(ToolCall {
            name: "Bash".into(),
            detail: Some("cargo test --all".into()),
        })
    );
}

#[test]
fn an_edit_shows_the_file_and_one_line_of_it() {
    let body = br#"{
        "hook_event_name": "PostToolUse",
        "tool_name": "Edit",
        "tool_input": { "file_path": "crates/ui/ui/src/lib.rs", "old_string": "a\nb" }
    }"#;
    let post = post("s", body).expect("a hook");
    let tool = post.tool.expect("a tool");
    assert_eq!(tool.name, "Edit");
    assert_eq!(tool.detail.as_deref(), Some("crates/ui/ui/src/lib.rs"));
}

#[test]
fn a_long_command_is_cut_to_one_short_line() {
    let long = "x".repeat(200);
    let body = format!(
        r#"{{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{{"command":"{long}\nmore"}}}}"#
    );
    let post = post("s", body.as_bytes()).expect("a hook");
    let detail = post.tool.and_then(|t| t.detail).expect("a detail");
    assert_eq!(detail.chars().count(), 80);
}

#[test]
fn a_hook_without_a_tool_is_still_a_hook() {
    let body = br#"{"hook_event_name": "Stop", "session_id": "claude-uuid"}"#;
    let post = post("s", body).expect("a hook");
    assert_eq!(post.kind, HookKind::Stop);
    assert!(post.tool.is_none());
}

#[test]
fn what_is_not_a_hook_is_refused() {
    assert!(post("s", b"not json").is_none());
    assert!(post("s", br#"{"hook_event_name": "Whatever"}"#).is_none());
    assert!(post("s", b"{}").is_none());
}
