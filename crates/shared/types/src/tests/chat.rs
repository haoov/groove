//! The conversation's rows, as the transcript's records make them.

use crate::chat::{Chat, Entry, Glance, Heard, Outcome, Patch, Run};

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

fn look(at: &str) -> Heard {
    Heard::Look {
        id: at.into(),
        at: at.into(),
    }
}

#[test]
fn thinking_looks_and_commands_back_to_back_fold_into_one_glance() {
    let chat = heard([
        Heard::Thought,
        look("service.py"),
        ran("ls"),
        look("client.py"),
        ran("pytest -q"),
        Heard::Said("done".into()),
        Heard::Thought,
    ]);
    let looked = vec!["service.py".to_string(), "client.py".to_string()];
    assert_eq!(
        chat.entries(),
        [
            Entry::Glance(Glance {
                thoughts: 1,
                looked,
                ran: vec![run("ls"), run("pytest -q")],
                running: true,
                doing: Some("pytest -q".into()),
                ..Glance::default()
            }),
            Entry::Said("done".into()),
            Entry::Glance(Glance {
                thoughts: 1,
                ..Glance::default()
            }),
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
    assert_eq!(glance.ran[0].output.as_deref(), Some("1 failed"));
    assert!(glance.ran[0].failed);

    chat.hear(Heard::Queued("go on".into()));
    chat.hear(Heard::Queued("go on".into()));
    assert_eq!(
        chat.queued(),
        ["go on"],
        "the echo and Claude's own queue are one"
    );
    chat.hear(Heard::You("go on".into()));
    assert!(chat.queued().is_empty());
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
    let called = &glance.ran[1];
    assert_eq!(called.tool.as_deref(), Some("mcp__groove__git_commit"));
    assert_eq!(called.command, "feat: x");
    assert_eq!(alone.name, "WebFetch");
    assert_eq!(glance.doing.as_deref(), Some("git commit"));
}
