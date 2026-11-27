//! The conversation's rows, as the transcript's records make them.

use crate::chat::{Chat, Entry, Glance, Heard, Kind, Outcome, Patch, Run, Step};

fn heard(said: impl IntoIterator<Item = Heard>) -> Chat {
    let mut chat = Chat::default();
    said.into_iter().for_each(|one| chat.hear(one));
    chat
}

fn ran(command: &str) -> Heard {
    Heard::Ran {
        id: command.into(),
        command: command.into(),
        about: None,
    }
}

fn run(command: &str) -> Run {
    Run {
        id: command.into(),
        tool: None,
        command: command.into(),
        output: None,
        failed: false,
    }
}

fn step(kind: Kind, target: &str, run: Option<Run>) -> Step {
    Step {
        kind,
        target: target.into(),
        run,
        at: None,
    }
}

fn look(at: &str) -> Heard {
    Heard::Look(at.into())
}

#[test]
fn looks_and_commands_back_to_back_fold_into_one_glance_and_a_thought_adds_nothing() {
    let chat = heard([
        Heard::Thought,
        look("service.py"),
        ran("ls"),
        look("client.py"),
        ran("pytest -q"),
        Heard::Said("done".into()),
        Heard::Thought,
    ]);
    let steps = vec![
        step(Kind::Read, "service.py", None),
        step(Kind::Ran, "ls", Some(run("ls"))),
        step(Kind::Read, "client.py", None),
        step(Kind::Ran, "pytest -q", Some(run("pytest -q"))),
    ];
    assert_eq!(
        chat.entries(),
        [
            Entry::Glance(Glance {
                steps,
                running: true,
                doing: Some("pytest -q".into()),
                ..Glance::default()
            }),
            Entry::Said("done".into()),
        ]
    );
}

#[test]
fn a_result_lands_on_the_call_it_answers() {
    let patch = Patch {
        added: 14,
        removed: 3,
        line: 16,
    };
    let chat = heard([
        Heard::Edit {
            id: "a".into(),
            path: "/w/service.py".into(),
            created: false,
        },
        Heard::Tool {
            id: "b".into(),
            name: "WebFetch".into(),
            detail: None,
        },
        Heard::Result {
            id: "b".into(),
            outcome: Outcome {
                failed: true,
                ..Outcome::default()
            },
        },
        Heard::Result {
            id: "a".into(),
            outcome: Outcome {
                patch: Some(patch),
                ..Outcome::default()
            },
        },
    ]);
    let [Entry::Edit(edit), Entry::Tool(tool)] = chat.entries() else {
        panic!("{:?}", chat.entries());
    };
    assert_eq!(edit.patch, Some(patch));
    assert_eq!(tool.failed, Some(true));
}

#[test]
fn a_result_for_no_call_held_changes_nothing() {
    let chat = heard([
        Heard::You("hi".into()),
        Heard::Result {
            id: "gone".into(),
            outcome: Outcome::default(),
        },
    ]);
    assert_eq!(chat.entries(), [Entry::You("hi".into())]);
}

#[test]
fn a_message_waits_after_the_rows_until_its_turn_takes_it() {
    let mut chat = heard([
        Heard::Queued("also this".into()),
        Heard::Queued("and that".into()),
        Heard::Said("working".into()),
    ]);
    assert_eq!(chat.queued(), ["also this", "and that"]);
    chat.hear(Heard::Dequeued(Some("and that".into())));
    chat.hear(Heard::Dequeued(None));
    assert!(chat.queued().is_empty());
    chat.hear(Heard::Queued("again".into()));
    chat.hear(Heard::Recalled);
    assert!(chat.queued().is_empty());
}

#[test]
fn a_glance_lasts_from_its_first_record_to_the_one_after_its_last() {
    let at = crate::Timestamp::new;
    let chat = heard([
        Heard::Clock(at(100)),
        Heard::Thought,
        Heard::Clock(at(104)),
        ran("cargo test"),
        Heard::Clock(at(130)),
        Heard::Said("done".into()),
    ]);
    let Entry::Glance(glance) = &chat.entries()[0] else {
        panic!("{:?}", chat.entries());
    };
    assert_eq!((glance.since, glance.until), (Some(at(100)), Some(at(130))));
}

#[test]
fn a_command_keeps_what_it_printed_and_a_sent_message_waits_until_the_transcript_says_it() {
    let mut chat = heard([ran("pytest")]);
    let outcome = Outcome {
        failed: true,
        output: Some("1 failed".into()),
        patch: None,
    };
    chat.hear(Heard::Result {
        id: "pytest".into(),
        outcome,
    });
    let Entry::Glance(glance) = &chat.entries()[0] else {
        panic!("{:?}", chat.entries());
    };
    let run = glance.runs().next().expect("its command");
    assert_eq!(run.output.as_deref(), Some("1 failed"));
    assert!(run.failed);

    chat.hear(Heard::Sent("go on".into()));
    chat.hear(Heard::Queued("go on".into()));
    assert_eq!(chat.queued(), ["go on"]);
    assert!(
        chat.sent().is_empty(),
        "the echo and Claude's own queue are one"
    );
    chat.hear(Heard::Queued("and then".into()));
    chat.hear(Heard::Sent("a long paste".into()));
    chat.hear(Heard::You("go on".into()));
    assert_eq!(chat.queued(), ["and then"]);
    chat.hear(Heard::You("a long paste, as Claude wrote it".into()));
    assert!(
        chat.sent().is_empty(),
        "the oldest sent goes when nothing says the same"
    );
    assert_eq!(
        chat.queued(),
        ["and then"],
        "a message Claude queued waits on"
    );
}

#[test]
fn an_mcp_call_folds_into_the_glance_and_another_tool_stands_alone() {
    let tool = |id: &str, name: &str| Heard::Tool {
        id: id.into(),
        name: name.into(),
        detail: Some("feat: x".into()),
    };
    let chat = heard([
        ran("ls"),
        tool("g", "mcp__groove__git_commit"),
        tool("w", "WebFetch"),
    ]);
    let [Entry::Glance(glance), Entry::Tool(alone)] = chat.entries() else {
        panic!("{:?}", chat.entries());
    };
    let called = glance.runs().nth(1).expect("the call");
    assert_eq!(glance.steps[1].kind, Kind::Called);
    assert_eq!(glance.steps[1].target, "groove · git_commit");
    assert_eq!(called.tool.as_deref(), Some("mcp__groove__git_commit"));
    assert_eq!(called.command, "feat: x");
    assert_eq!(alone.name, "WebFetch");
    assert_eq!(glance.doing.as_deref(), Some("git commit"));
}

fn sent(id: &str) -> Heard {
    Heard::Subagent {
        id: id.into(),
        kind: "Explore".into(),
        task: "Find the metrics".into(),
    }
}

fn subagent(chat: &Chat, at: usize) -> &crate::chat::Subagent {
    match &chat.entries()[at] {
        Entry::Subagent(sub) => sub,
        other => panic!("a subagent, not {other:?}"),
    }
}

#[test]
fn a_subagent_folds_its_own_steps_and_its_answer_ends_it() {
    let within = Heard::Within {
        parent: "a1".into(),
        heard: vec![Heard::Thought, look("observability.py"), ran("rg Counter")],
    };
    let answer = Heard::Result {
        id: "a1".into(),
        outcome: Outcome {
            output: Some("They register in observability.py".into()),
            ..Outcome::default()
        },
    };
    let chat = heard([sent("a1"), within.clone(), Heard::Said("meanwhile".into())]);
    let sub = subagent(&chat, 0);
    assert_eq!(Kind::ALL.map(|kind| sub.steps.count(kind)), [1, 0, 1, 0]);
    assert_eq!(sub.steps.doing.as_deref(), Some("rg Counter"));
    assert_eq!(sub.state, crate::chat::Sent::Working);
    let chat = heard([sent("a1"), within, answer]);
    let sub = subagent(&chat, 0);
    assert_eq!(sub.state, crate::chat::Sent::Done);
    assert_eq!(
        sub.answer.as_deref(),
        Some("They register in observability.py")
    );
}

#[test]
fn an_interrupt_stops_a_working_subagent_and_a_failure_is_a_row() {
    let chat = heard([
        sent("a1"),
        Heard::Interrupted,
        Heard::Failed("API Error: overloaded".into()),
    ]);
    assert_eq!(subagent(&chat, 0).state, crate::chat::Sent::Stopped);
    assert_eq!(
        chat.entries()[2],
        Entry::Failed("API Error: overloaded".into())
    );
}

#[test]
fn a_model_reads_by_its_name_and_tokens_and_resets_read_short() {
    assert_eq!(crate::chat::model_name("claude-opus-5-5"), "opus 5.5");
    assert_eq!(crate::chat::model_name("claude-fable-5-1"), "fable 5.1");
    assert_eq!(crate::chat::tokens(82_002), "82k");
    assert_eq!(crate::chat::tokens(1_000_000), "1M");
    assert_eq!(crate::chat::tokens(1_500_000), "1.5M");
    let now = crate::Timestamp::new(0);
    assert_eq!(
        crate::chat::resets(crate::Timestamp::new(6_120), now),
        "resets in 1h 42m"
    );
    assert_eq!(
        crate::chat::resets(crate::Timestamp::new(270_000), now),
        "resets in 3d 3h"
    );
}

#[test]
fn each_step_lasts_until_the_next_and_each_row_keeps_when_it_was_said() {
    use crate::Timestamp;
    let chat = heard([
        Heard::Clock(Timestamp::new(100)),
        Heard::You("go".into()),
        Heard::Clock(Timestamp::new(102)),
        look("service.py"),
        Heard::Clock(Timestamp::new(106)),
        Heard::Searched("Counter".into()),
        Heard::Clock(Timestamp::new(107)),
    ]);
    let Entry::Glance(glance) = &chat.entries()[1] else {
        panic!("{:?}", chat.entries());
    };
    let took = |at| glance.took(at).map(|one| one.as_secs());
    assert_eq!((took(0), took(1)), (Some(4), Some(1)));
    assert_eq!(glance.steps[1].kind, Kind::Searched);
    assert_eq!(chat.stamp(0), Some(Timestamp::new(100)));
}
