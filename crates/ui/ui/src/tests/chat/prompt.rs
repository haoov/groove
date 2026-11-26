//! The prompt over everything: opened and closed on alt+a, closed by Escape with its draft kept,
//! open while the keys go elsewhere.

use groove_controllers::{Command, agent};
use groove_gfx::Fonts;
use groove_types::SessionId;

use super::{chat_ui, talking};
use crate::hit::Target;
use crate::input::{Key, Modifiers};
use crate::tests::{ALT, all_texts, click, press, typed, window};

#[test]
fn alt_a_opens_the_prompt_and_alt_a_again_closes_it() {
    let app = talking(Vec::new());
    let mut ui = chat_ui();
    press(Key::Char('a'), ALT, &mut ui, &app);
    assert!(ui.prompting());
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(hits.rect_of(&Target::Prompt).is_some(), "drawn over everything");
    press(Key::Char('a'), ALT, &mut ui, &app);
    assert!(!ui.prompting());
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(hits.rect_of(&Target::Prompt).is_none(), "the pane rests without one");
}

#[test]
fn escape_closes_the_prompt_and_keeps_its_draft() {
    let app = talking(Vec::new());
    let mut ui = chat_ui();
    press(Key::Char('a'), ALT, &mut ui, &app);
    typed("half a thought", &mut ui, &app);
    assert!(press(Key::Escape, Modifiers::default(), &mut ui, &app).is_empty());
    assert!(!ui.prompting(), "closed, nothing interrupted");
    press(Key::Char('a'), ALT, &mut ui, &app);
    assert_eq!(ui.agent.prompt.text(), "half a thought");
}

#[test]
fn enter_sends_the_message_and_closes_the_prompt() {
    let app = talking(Vec::new());
    let mut ui = chat_ui();
    press(Key::Char('a'), ALT, &mut ui, &app);
    typed("go on", &mut ui, &app);
    let sent = press(Key::Enter, Modifiers::default(), &mut ui, &app);
    let say = agent::Command::Say {
        session: SessionId::new("a"),
        text: "go on".into(),
    };
    assert_eq!(sent, [Command::Agent(say)]);
    assert!(!ui.prompting());
    assert!(ui.agent.prompt.is_empty());
}

#[test]
fn the_prompt_stays_open_while_the_keys_go_elsewhere_and_alt_a_takes_them_back() {
    let app = talking(vec![groove_types::chat::Heard::Said("done".into())]);
    let mut ui = chat_ui();
    press(Key::Char('a'), ALT, &mut ui, &app);
    typed("see ", &mut ui, &app);
    press(Key::Char('r'), ALT, &mut ui, &app);
    assert!(ui.prompting(), "open while the rail has the keys");
    typed("x", &mut ui, &app);
    assert_eq!(ui.agent.prompt.text(), "see ", "the keys went to the rail");
    press(Key::Char('a'), ALT, &mut ui, &app);
    assert!(ui.prompting(), "alt+a gives the keys back first");
    typed("this", &mut ui, &app);
    assert_eq!(ui.agent.prompt.text(), "see this");
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let prompt = hits.rect_of(&Target::Prompt).expect("the prompt");
    let _ = click(prompt, &mut ui, &app, &hits);
    assert!(ui.prompting(), "a click on it keeps it");
}

#[test]
fn at_rest_the_pane_says_how_to_reply() {
    let app = talking(Vec::new());
    let (frame, _) = crate::view(&app, &chat_ui(), window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    for said in ["ALT", "A", "reply"] {
        assert!(texts.iter().any(|one| one == said), "{said}: {texts:?}");
    }
}

#[test]
fn with_the_prompt_open_the_wheel_still_scrolls_the_transcript() {
    use crate::input::{Delta, Input, handle};
    let said = (0..60).map(|at| groove_types::chat::Heard::Said(format!("step {at} done")));
    let app = talking(said.collect());
    let mut ui = chat_ui();
    press(Key::Char('a'), ALT, &mut ui, &app);
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let pane = crate::layout::Layout::of(window(), &ui).agent;
    let wheel = Input::Scroll {
        x: pane.x + pane.w / 2.0,
        y: pane.y + pane.h / 4.0,
        delta: Delta::Lines {
            across: 0.0,
            down: 3.0,
        },
    };
    handle(wheel, &mut ui, &app, &hits, window());
    assert!(ui.agent.scroll > 0.0, "the transcript moved");
    assert!(ui.prompting());
}
