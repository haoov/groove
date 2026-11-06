//! The agent's stream: what Groove writes to it, and what it hears back.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use groove_agent_launch::Launch;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream};
use tokio::sync::mpsc;

use crate::{Heard, Hooks, Stream};

const WAIT: Duration = Duration::from_secs(5);

/// A stream on in-memory pipes: the agent's side to write and read, and what Groove heard.
struct Scene {
    stream: Stream,
    agent_out: DuplexStream,
    agent_in: tokio::io::Lines<BufReader<DuplexStream>>,
    heard: mpsc::UnboundedReceiver<Heard>,
}

fn scene() -> Scene {
    let (agent_out, groove_in) = tokio::io::duplex(1 << 16);
    let (groove_out, agent_in) = tokio::io::duplex(1 << 16);
    let (tell, heard) = mpsc::unbounded_channel();
    let on_heard = Box::new(move |one| drop(tell.send(one)));
    Scene {
        stream: Stream::connect(groove_in, groove_out, on_heard),
        agent_out,
        agent_in: BufReader::new(agent_in).lines(),
        heard,
    }
}

impl Scene {
    async fn says(&mut self, record: Value) {
        let line = format!("{record}\n");
        self.agent_out
            .write_all(line.as_bytes())
            .await
            .expect("written");
    }

    async fn hears(&mut self) -> Heard {
        let next = tokio::time::timeout(WAIT, self.heard.recv()).await;
        next.expect("in time").expect("a record")
    }

    async fn reads(&mut self) -> Value {
        let next = tokio::time::timeout(WAIT, self.agent_in.next_line()).await;
        let line = next.expect("in time").expect("read").expect("a line");
        serde_json::from_str(&line).expect("json")
    }
}

fn asks(id: &str, tool: &str, input: Value) -> Value {
    json!({"type": "control_request", "request_id": id,
        "request": {"subtype": "can_use_tool", "tool_name": tool, "input": input}})
}

#[tokio::test]
async fn a_tool_s_ask_is_heard_and_the_leave_goes_back_with_its_input() {
    let mut scene = scene();
    let input = json!({"command": "cargo test\nmore", "description": "run the tests"});
    scene.says(asks("r1", "Bash", input.clone())).await;
    let Heard::Asks(asked) = scene.hears().await else {
        panic!("an ask");
    };
    assert_eq!(
        (asked.tool.as_str(), asked.detail.as_deref()),
        ("Bash", Some("cargo test"))
    );
    scene
        .stream
        .allow(&asked.id, asked.input.clone())
        .expect("sent");
    let answer = scene.reads().await;
    assert_eq!(answer["response"]["request_id"], "r1");
    assert_eq!(answer["response"]["response"]["behavior"], "allow");
    assert_eq!(answer["response"]["response"]["updatedInput"], input);
    scene.stream.deny("r2", "not now").expect("sent");
    let refused = scene.reads().await;
    assert_eq!(refused["response"]["response"]["message"], "not now");
}

#[tokio::test]
async fn a_question_s_options_are_read_and_the_pick_answers_it() {
    let mut scene = scene();
    let question = json!({"question": "Tea or coffee?", "header": "Drink", "multiSelect": false,
        "options": [{"label": "Tea", "description": "hot"}, {"label": "Coffee"}]});
    scene
        .says(asks(
            "q1",
            "AskUserQuestion",
            json!({"questions": [question]}),
        ))
        .await;
    let Heard::Asks(asked) = scene.hears().await else {
        panic!("an ask");
    };
    let one = &asked.questions[0];
    assert_eq!(
        (one.header.as_str(), one.options.clone()),
        ("Drink", vec!["Tea".into(), "Coffee".into()])
    );
    let input = asked.answered(&["Coffee".into()]);
    assert_eq!(input["answers"], json!({"Tea or coffee?": "Coffee"}));
    assert_eq!(input["questions"][0]["header"], "Drink");
}

#[tokio::test]
async fn words_and_an_interrupt_are_written_one_line_each() {
    let mut scene = scene();
    scene.stream.say("cache the quotes").expect("sent");
    scene.stream.interrupt().expect("sent");
    let said = scene.reads().await;
    assert_eq!(
        (said["type"].as_str(), said["message"]["content"].as_str()),
        (Some("user"), Some("cache the quotes"))
    );
    let stopped = scene.reads().await;
    assert_eq!(stopped["request"]["subtype"], "interrupt");
}

#[tokio::test]
async fn a_turn_s_end_and_a_withdrawn_ask_are_heard_and_an_unknown_request_is_refused() {
    let mut scene = scene();
    let unknown = json!({"type": "control_request", "request_id": "h1", "request": {"subtype": "hook_callback"}});
    scene
        .says(json!({"type": "assistant", "message": {"content": []}}))
        .await;
    scene.says(unknown).await;
    scene
        .says(json!({"type": "control_cancel_request", "request_id": "r1"}))
        .await;
    scene
        .says(json!({"type": "result", "subtype": "success"}))
        .await;
    assert_eq!(scene.hears().await, Heard::Withdrawn("r1".into()));
    assert_eq!(
        scene.hears().await,
        Heard::Settled {
            windows: Vec::new()
        }
    );
    let refused = scene.reads().await;
    assert_eq!(
        (
            refused["response"]["subtype"].as_str(),
            refused["response"]["request_id"].as_str()
        ),
        (Some("error"), Some("h1"))
    );
}

#[tokio::test]
async fn a_spawned_agent_reads_its_input_and_its_end_says_its_code_and_last_stderr() {
    let script = r#"read line; case "$line" in *quotes*) echo '{"type":"result"}';; esac; echo 'not logged in' >&2; exit 3"#;
    let launch = Launch {
        program: "sh".into(),
        args: vec!["-c".into(), script.into()],
        env: Vec::new(),
        cwd: std::env::temp_dir(),
    };
    let heard = Arc::new(Mutex::new(Vec::new()));
    let (exited, code) = tokio::sync::oneshot::channel();
    let kept = heard.clone();
    let hooks = Hooks {
        on_heard: Box::new(move |one| kept.lock().expect("held").push(one)),
        on_exit: Box::new(move |one| {
            let _ = exited.send(one);
        }),
    };
    let stream = Stream::spawn(launch, hooks).await.expect("spawned");
    stream.say("cache the quotes").expect("sent");
    let code = tokio::time::timeout(WAIT, code).await.expect("in time");
    assert_eq!(code.expect("an exit"), 3);
    tokio::time::sleep(Duration::from_millis(50)).await;
    let ended = "The agent ended with code 3:\nnot logged in".to_string();
    let ended = Heard::Chat(groove_types::chat::Heard::Failed(ended));
    assert_eq!(
        *heard.lock().expect("held"),
        [
            Heard::Settled {
                windows: Vec::new()
            },
            ended
        ]
    );
}

#[tokio::test]
async fn a_subagent_s_own_records_the_plan_s_use_and_a_refusing_limit_are_heard() {
    let mut scene = scene();
    let step = json!({"type": "assistant", "parent_tool_use_id": "a1",
        "message": {"content": [{"type": "thinking", "thinking": ""}]}});
    let main = json!({"type": "assistant", "parent_tool_use_id": null,
        "message": {"content": [{"type": "text", "text": "the transcript says this"}]}});
    let allowed = json!({"type": "rate_limit_event", "rate_limit_info": {"status": "allowed",
        "unifiedWindows": {"five_hour": {"utilization": 0.48}, "seven_day": {"utilization": 0.09}}}});
    let refused = json!({"type": "rate_limit_event",
        "rate_limit_info": {"status": "rejected", "rateLimitType": "five_hour"}});
    for one in [step, main, allowed, refused] {
        scene.says(one).await;
    }
    let within = groove_types::chat::Heard::Within {
        parent: "a1".into(),
        heard: vec![groove_types::chat::Heard::Thought],
    };
    assert_eq!(scene.hears().await, Heard::Chat(within));
    let limits = groove_types::chat::Limits {
        five_hour: Some(groove_types::chat::Window {
            used: 0.48,
            resets: None,
        }),
        week: Some(groove_types::chat::Window {
            used: 0.09,
            resets: None,
        }),
    };
    let used = Heard::Limits {
        limits,
        refused: None,
    };
    assert_eq!(scene.hears().await, used);
    let refused = Some("The five-hour limit is reached.".to_string());
    let limits = groove_types::chat::Limits::default();
    assert_eq!(scene.hears().await, Heard::Limits { limits, refused });
}

#[tokio::test]
async fn a_model_switch_is_written_as_a_control_request() {
    let mut scene = scene();
    scene.stream.set_model("claude-sonnet-5-5").expect("sent");
    let asked = scene.reads().await;
    assert_eq!(asked["request"]["subtype"], "set_model");
    assert_eq!(asked["request"]["model"], "claude-sonnet-5-5");
}
