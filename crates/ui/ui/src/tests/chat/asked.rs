//! What a headless agent asks, answered from its card or the keys of an empty prompt; Escape alone interrupts.

use groove_controllers::agent_service::{Event, StreamHeard, apply};
use groove_controllers::{AppState, Command, agent};
use groove_gfx::Fonts;
use groove_types::chat::{Permission, Question};
use groove_types::{SessionId, Timestamp};

use super::{chat_ui, talking};
use crate::hit::{Answering, Target};
use crate::input::{Key, Modifiers};
use crate::tests::{click, press, typed, window};

fn asked(app: &mut AppState, questions: Vec<Question>) {
    let launch = app.agent.agents[0].1.launch;
    let permission = Permission {
        id: "r1".into(),
        tool: "Bash".into(),
        detail: Some("cargo test".into()),
        questions,
        input: serde_json::json!({"command": "cargo test"}),
    };
    let event = Event::Streamed {
        session: SessionId::new("a"),
        launch,
        heard: StreamHeard::Asks(permission),
        at: Timestamp::new(1),
    };
    apply(&mut app.agent, event);
}

fn answer(allow: bool, picked: &[&str]) -> Vec<Command> {
    let picked = picked.iter().map(|one| one.to_string()).collect();
    let id = "r1".into();
    let session = SessionId::new("a");
    vec![Command::Agent(agent::Command::Answer {
        session,
        id,
        allow,
        picked,
    })]
}

fn question(options: &[&str]) -> Question {
    Question {
        question: "Which?".into(),
        header: "Pick".into(),
        options: options.iter().map(|one| one.to_string()).collect(),
    }
}

#[test]
fn a_tool_s_ask_is_let_or_refused_from_its_card_and_from_an_empty_prompt() {
    let mut app = talking(Vec::new());
    asked(&mut app, Vec::new());
    let mut ui = chat_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let allow = hits
        .rect_of(&Target::Answer(Answering::Allow))
        .expect("allow");
    assert_eq!(click(allow, &mut ui, &app, &hits), answer(true, &[]));
    let refuse = hits
        .rect_of(&Target::Answer(Answering::Refuse))
        .expect("refuse");
    assert_eq!(click(refuse, &mut ui, &app, &hits), answer(false, &[]));
    let plain = Modifiers::default();
    assert_eq!(press(Key::Enter, plain, &mut ui, &app), answer(true, &[]));
    assert_eq!(press(Key::Escape, plain, &mut ui, &app), answer(false, &[]));
    typed("go", &mut ui, &app);
    let said = press(Key::Enter, plain, &mut ui, &app);
    assert!(
        matches!(said[..], [Command::Agent(agent::Command::Say { .. })]),
        "{said:?}"
    );
}

#[test]
fn a_question_s_options_are_picked_one_question_at_a_time_then_sent_together() {
    let mut app = talking(Vec::new());
    asked(
        &mut app,
        vec![question(&["Tea", "Coffee"]), question(&["Now", "Later"])],
    );
    let mut ui = chat_ui();
    let plain = Modifiers::default();
    assert!(
        press(Key::Enter, plain, &mut ui, &app).is_empty(),
        "a question needs a pick"
    );
    assert!(press(Key::Char('2'), plain, &mut ui, &app).is_empty());
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let later = Target::Answer(Answering::Pick("Later".into()));
    let later = hits.rect_of(&later).expect("the second question's option");
    assert_eq!(
        click(later, &mut ui, &app, &hits),
        answer(true, &["Coffee", "Later"])
    );
    assert_eq!(ui.agent.picked, None);
}

#[test]
fn escape_on_an_empty_prompt_interrupts_the_agent_when_nothing_is_asked() {
    let app = talking(Vec::new());
    let mut ui = chat_ui();
    let stop = press(Key::Escape, Modifiers::default(), &mut ui, &app);
    let session = SessionId::new("a");
    assert_eq!(
        stop,
        [Command::Agent(agent::Command::Interrupt { session })]
    );
}
