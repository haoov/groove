//! Transcript lines read into what the conversation hears.

use groove_types::chat::{Heard, Outcome, Patch};
use serde_json::json;

use crate::line;

fn said(record: serde_json::Value) -> Vec<Heard> {
    line(&record.to_string())
}

fn assistant(blocks: serde_json::Value) -> serde_json::Value {
    json!({"type": "assistant", "message": {"role": "assistant", "content": blocks}})
}

#[test]
fn the_user_s_prompt_and_a_slash_command_are_heard_as_typed() {
    let typed = json!({"type": "user", "message": {"content": "cache the quotes"}});
    let command = json!({"type": "user", "message": {"content":
        "<command-message>x</command-message>\n<command-name>/groove:save-task</command-name>"}});
    assert_eq!(said(typed), [Heard::You("cache the quotes".into())]);
    assert_eq!(said(command), [Heard::You("/groove:save-task".into())]);
}

#[test]
fn an_interrupt_is_heard_as_one_and_not_as_the_user_s_words() {
    let stopped = |text: &str| json!({"type": "user", "message": {"content": [{"type": "text", "text": text}]}});
    assert_eq!(
        said(stopped("[Request interrupted by user]")),
        [Heard::Interrupted]
    );
    let in_tool = stopped("[Request interrupted by user for tool use]");
    assert_eq!(said(in_tool), [Heard::Interrupted]);
}

#[test]
fn meta_and_side_chain_records_say_nothing() {
    let meta = json!({"type": "user", "isMeta": true, "message": {"content": "skill body"}});
    let side = json!({"type": "assistant", "isSidechain": true,
        "message": {"content": [{"type": "text", "text": "sub agent"}]}});
    assert!(said(meta).is_empty());
    assert!(said(side).is_empty());
}

#[test]
fn a_record_kind_groove_does_not_know_is_skipped() {
    assert!(said(json!({"type": "ai-title", "aiTitle": "x"})).is_empty());
    assert!(line("not json").is_empty());
}

#[test]
fn the_agent_s_blocks_are_heard_by_what_they_do() {
    let heard = said(assistant(json!([
        {"type": "thinking", "thinking": ""},
        {"type": "text", "text": "I add a cache."},
        {"type": "tool_use", "id": "r", "name": "Read", "input": {"file_path": "/w/src/service.py"}},
        {"type": "tool_use", "id": "w", "name": "Write", "input": {"file_path": "/w/cache.py"}},
        {"type": "tool_use", "id": "e", "name": "Edit", "input": {"file_path": "/w/service.py"}},
        {"type": "tool_use", "id": "b", "name": "Bash",
            "input": {"command": "pytest -q", "description": "Run the tests"}},
        {"type": "tool_use", "id": "s", "name": "ToolSearch", "input": {"query": "x"}},
        {"type": "tool_use", "id": "g", "name": "mcp__groove__git_commit",
            "input": {"message": "feat: cache\n\nbody"}},
    ])));
    let edit = |id: &str, path: &str, created| Heard::Edit {
        id: id.into(),
        path: path.into(),
        created,
    };
    assert_eq!(
        heard,
        [
            Heard::Thought,
            Heard::Said("I add a cache.".into()),
            Heard::Look("service.py".into()),
            edit("w", "/w/cache.py", true),
            edit("e", "/w/service.py", false),
            Heard::Ran {
                id: "b".into(),
                command: "pytest -q".into(),
                about: Some("Run the tests".into())
            },
            Heard::Tool {
                id: "g".into(),
                name: "mcp__groove__git_commit".into(),
                detail: Some("feat: cache".into())
            },
        ]
    );
}

#[test]
fn an_edit_s_result_counts_the_lines_its_patch_gains_and_loses() {
    let record = json!({"type": "user",
    "message": {"content": [{"type": "tool_result", "tool_use_id": "e", "content": "ok"}]},
    "toolUseResult": {"filePath": "/w/service.py", "structuredPatch": [
        {"oldStart": 13, "newStart": 16, "lines": [" a", "-b", "+c", "+d"]},
        {"oldStart": 40, "newStart": 44, "lines": ["+e"]}
    ]}});
    let heard = said(record);
    let [Heard::Result { id, outcome }] = heard.as_slice() else {
        panic!("{heard:?}");
    };
    assert_eq!(id, "e");
    let patch = Patch {
        added: 3,
        removed: 1,
        line: 16,
    };
    assert_eq!(outcome.patch, Some(patch));
}

#[test]
fn a_new_file_gains_every_line_it_holds() {
    let record = json!({"type": "user",
        "message": {"content": [{"type": "tool_result", "tool_use_id": "w", "content": "ok"}]},
        "toolUseResult": {"type": "create", "content": "a\nb\nc\n", "structuredPatch": []}});
    let heard = said(record);
    let Some(Heard::Result { outcome, .. }) = heard.first() else {
        panic!("{heard:?}");
    };
    assert_eq!(outcome.patch.map(|one| one.added), Some(3));
}

#[test]
fn a_call_s_result_says_whether_it_failed_and_what_it_printed() {
    let record = json!({"type": "user",
        "message": {"content": [{"type": "tool_result", "tool_use_id": "b", "is_error": true,
            "content": "Exit code 1"}]},
        "toolUseResult": {"stdout": "1 failed", "stderr": "", "interrupted": false}});
    let expected = Outcome {
        failed: true,
        output: Some("1 failed".into()),
        patch: None,
    };
    assert_eq!(
        said(record),
        [Heard::Result {
            id: "b".into(),
            outcome: expected
        }]
    );
}

#[test]
fn a_pasted_prompt_and_one_typed_while_the_agent_works_are_heard() {
    let pasted = json!({"type": "user", "message": {"content":
        "\n\n<pasted_content id=\"d3\">\nfix the prompt\n</pasted_content id=\"d3\">"}});
    let queued = json!({"type": "attachment", "attachment":
        {"type": "queued_command", "prompt": "also the bubbles", "commandMode": "prompt"}});
    assert_eq!(said(pasted), [Heard::You("fix the prompt".into())]);
    assert_eq!(said(queued), [Heard::You("also the bubbles".into())]);
}

#[test]
fn a_message_typed_while_the_agent_works_waits_until_its_turn_takes_it() {
    let op = |operation: &str, content: Option<&str>| {
        let mut record = json!({"type": "queue-operation", "operation": operation});
        if let Some(content) = content {
            record["content"] = json!(content);
        }
        said(record)
    };
    assert_eq!(
        op("enqueue", Some("also this")),
        [Heard::Queued("also this".into())]
    );
    let taken = Heard::Dequeued(Some("also this".into()));
    assert_eq!(op("remove", Some("also this")), [taken]);
    assert_eq!(op("dequeue", None), [Heard::Dequeued(None)]);
    assert_eq!(op("popAll", None), [Heard::Recalled]);
}

#[test]
fn a_record_s_time_comes_before_what_it_says() {
    let record = json!({"type": "user", "timestamp": "2026-10-29T12:49:13.042Z",
        "message": {"content": "go"}});
    let at = groove_types::Timestamp::parse("2026-10-29T12:49:13Z").unwrap();
    assert_eq!(said(record), [Heard::Clock(at), Heard::You("go".into())]);
}

#[test]
fn an_agent_sent_is_a_subagent_its_handback_its_answer_and_an_api_error_a_failure() {
    let sent = assistant(json!([{"type": "tool_use", "id": "a1", "name": "Agent",
        "input": {"subagent_type": "Explore", "description": "Find the metrics", "prompt": "..."}}]));
    let sub = Heard::Subagent {
        id: "a1".into(),
        kind: "Explore".into(),
        task: "Find the metrics".into(),
    };
    assert_eq!(said(sent), [sub]);
    let back = json!({"type": "user", "toolUseResult": {"status": "completed",
        "handbackReport": {"text": "They register in observability.py\n"}},
        "message": {"content": [{"type": "tool_result", "tool_use_id": "a1",
            "content": [{"type": "text", "text": "delivered as a message"}]}]}});
    let outcome = Outcome {
        output: Some("They register in observability.py".into()),
        ..Outcome::default()
    };
    assert_eq!(
        said(back),
        [Heard::Result {
            id: "a1".into(),
            outcome
        }]
    );
    let error = json!({"type": "assistant", "isApiErrorMessage": true,
        "message": {"content": [{"type": "text", "text": "API Error: overloaded"}]}});
    assert_eq!(said(error), [Heard::Failed("API Error: overloaded".into())]);
}

#[test]
fn a_background_agent_s_launch_keeps_its_id_and_its_notification_ends_it() {
    let launched = json!({"type": "user",
        "toolUseResult": {"isAsync": true, "status": "async_launched", "agentId": "x"},
        "message": {"content": [{"type": "tool_result", "tool_use_id": "a1",
            "content": [{"type": "text", "text": "Async agent launched successfully."}]}]}});
    let background = Heard::Background {
        id: "a1".into(),
        agent: "x".into(),
    };
    assert_eq!(said(launched), [background]);
    let notice = "<task-notification>\n<task-id>x</task-id>\n<status>stopped</status>\n\
        <summary>It did not finish</summary>\n</task-notification>";
    let notified = json!({"type": "user", "message": {"content": notice}});
    let ended = Heard::Ended {
        call: None,
        agent: "x".into(),
        state: groove_types::chat::Sent::Stopped,
        summary: Some("It did not finish".into()),
    };
    assert_eq!(said(notified), [ended]);
}

#[test]
fn a_subagent_s_report_is_heard_from_its_message_and_its_queued_copy_is_not_a_prompt() {
    let message = "Another Claude session sent a message:\n<agent-message from=\"x\">\n\
        [Subagent hand-back] Framing. The report follows:\n  All green.\n  \n  1. fmt passes\n</agent-message>";
    let meta = json!({"type": "user", "isMeta": true, "message": {"content": message}});
    let reported = Heard::Reported {
        agent: "x".into(),
        report: "All green.\n\n1. fmt passes".into(),
    };
    assert_eq!(said(meta), [reported]);
    let copy = message.split_once('\n').map_or(message, |(_, rest)| rest);
    let queued = json!({"type": "queue-operation", "operation": "dequeue", "content": copy});
    assert_eq!(said(queued), []);
}

#[test]
fn a_skill_says_the_skill_it_runs_and_a_kube_call_its_kubectl_line() {
    let skill = assistant(json!([{"type": "tool_use", "id": "s1", "name": "Skill",
        "input": {"skill": "groove:save-task", "args": "notion"}}]));
    let ran = Heard::Tool {
        id: "s1".into(),
        name: "Skill".into(),
        detail: Some("groove:save-task notion".into()),
    };
    assert_eq!(said(skill), [ran]);
    let kube = assistant(
        json!([{"type": "tool_use", "id": "k1", "name": "mcp__groove__kube",
        "input": {"context": "hub", "args": ["get", "pods", "-n", "argo"]}}]),
    );
    let read = Heard::Tool {
        id: "k1".into(),
        name: "mcp__groove__kube".into(),
        detail: Some("kubectl --context hub get pods -n argo".into()),
    };
    assert_eq!(said(kube), [read]);
}

#[test]
fn an_answer_says_its_model_and_how_many_tokens_its_context_held() {
    let answer = json!({"type": "assistant", "message": {"model": "claude-opus-5-5",
        "usage": {"input_tokens": 2, "cache_creation_input_tokens": 1000, "cache_read_input_tokens": 81000},
        "content": [{"type": "text", "text": "done"}]}});
    let used = Heard::Used {
        model: "claude-opus-5-5".into(),
        context: 82002,
    };
    assert_eq!(said(answer), [Heard::Said("done".into()), used]);
}

#[test]
fn claude_code_s_own_synthetic_answer_says_no_model() {
    let answer = json!({"type": "assistant", "message": {"model": "<synthetic>",
        "usage": {"input_tokens": 0}, "content": [{"type": "text", "text": "No response requested."}]}});
    assert_eq!(said(answer), [Heard::Said("No response requested.".into())]);
}
