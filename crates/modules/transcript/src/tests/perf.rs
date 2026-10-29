//! What reading a long transcript costs: its lines parsed, then heard into a conversation. Run with
//! `cargo test --release -p groove-transcript perf -- --ignored --nocapture`.
#![allow(clippy::print_stdout)]

use std::time::Instant;

use groove_types::chat::Chat;
use serde_json::json;

/// One turn's records: the prompt, the agent's blocks, and what its tools gave back.
fn turn(at: usize) -> Vec<String> {
    let stamp = "2026-10-29T12:49:13.042Z";
    let prompt = json!({"type": "user", "timestamp": stamp,
        "message": {"content": format!("turn {at}: cache the quotes")}});
    let mut blocks = vec![json!({"type": "thinking", "thinking": "x".repeat(400)})];
    for n in 0..6 {
        blocks.push(
            json!({"type": "tool_use", "id": format!("b{at}-{n}"), "name": "Bash",
            "input": {"command": "cargo test --workspace", "description": "Run the tests"}}),
        );
    }
    blocks.push(json!({"type": "text", "text": "I add a cache. ".repeat(40)}));
    let said = json!({"type": "assistant", "timestamp": stamp, "message": {"content": blocks}});
    let mut out = vec![prompt.to_string(), said.to_string()];
    for n in 0..6 {
        let result = json!({"type": "user", "timestamp": stamp,
            "message": {"content": [{"type": "tool_result", "tool_use_id": format!("b{at}-{n}"),
                "content": "ok"}]},
            "toolUseResult": {"stdout": "line of output\n".repeat(60), "stderr": ""}});
        out.push(result.to_string());
    }
    out
}

#[test]
#[ignore]
fn time_reading_a_long_transcript() {
    for turns in [100, 1000, 5000] {
        let lines: Vec<String> = (0..turns).flat_map(turn).collect();
        let bytes: usize = lines.iter().map(String::len).sum();
        let started = Instant::now();
        let heard: Vec<_> = lines.iter().flat_map(|one| crate::line(one)).collect();
        let parsed = started.elapsed();
        let started = Instant::now();
        let mut chat = Chat::default();
        heard.into_iter().for_each(|one| chat.hear(one));
        let kept = chat.entries().len();
        println!(
            "{turns:>5} turns, {:>6} KiB: {parsed:>10?} parsed off the loop, {:>10?} heard on it, {kept} rows kept",
            bytes / 1024,
            started.elapsed()
        );
    }
}
