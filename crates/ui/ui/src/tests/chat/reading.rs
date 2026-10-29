//! The conversation read closely: a command's card opened, words taken by clicks, links followed.

use groove_controllers::{Command, task, workspace};
use groove_gfx::Fonts;
use groove_types::chat::{Heard, Outcome};
use groove_types::{AgentStatus, Caret, Selection};

use super::{CTRL, chat_ui, copied, ran, talking};
use crate::hit::Target;
use crate::input::{Input, Key, handle};
use crate::tests::{all_texts, click, press, pressed, release, typed, window};

fn printed(id: &str, output: &str) -> Heard {
    Heard::Result {
        id: id.into(),
        outcome: Outcome {
            output: Some(output.into()),
            ..Outcome::default()
        },
    }
}

#[test]
fn a_click_on_a_command_card_shows_every_command_whole_with_what_it_printed() {
    let app = talking(vec![
        ran("cargo test\n  --locked"),
        printed("cargo test\n  --locked", "test result: ok"),
    ]);
    let mut ui = chat_ui();
    let (frame, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(!all_texts(&frame).iter().any(|one| one == "test result: ok"));
    let card = Target::Commands("cargo test\n  --locked".into());
    let rect = hits.rect_of(&card).expect("the card");
    assert!(click(rect, &mut ui, &app, &hits).is_empty());
    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    for said in ["  --locked", "test result: ok"] {
        assert!(texts.iter().any(|one| one == said), "{said}: {texts:?}");
    }
}

/// Presses at one point, `count` of them, then lets go.
fn clicked(
    count: usize,
    at: (f32, f32),
    ui: &mut crate::Ui,
    app: &groove_controllers::AppState,
) -> Vec<Command> {
    let (_, hits) = crate::view(app, ui, window(), &mut Fonts::embedded());
    let mut out = Vec::new();
    for _ in 0..count {
        pressed(at.0, at.1, ui, app, &hits);
        out = release(ui, app, &hits);
    }
    out
}

#[test]
fn a_double_click_takes_a_word_and_a_triple_click_its_line() {
    let app = talking(vec![Heard::You("cache the quotes".into())]);
    let mut ui = chat_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let run = hits
        .read()
        .iter()
        .find(|one| one.text == "cache the quotes");
    let rect = run.expect("the bubble's words").rect;
    let on = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
    assert_eq!(clicked(2, on, &mut ui, &app), copied("the"));
    assert_eq!(clicked(3, on, &mut ui, &app), copied("cache the quotes"));
}

#[test]
fn a_double_click_in_the_prompt_takes_its_word() {
    let app = talking(Vec::new());
    let mut ui = chat_ui();
    typed("one two", &mut ui, &app);
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let (row, _) = hits.typed().rows[0];
    let on = (row.x + window().advance * 5.0, row.y + row.h / 2.0);
    clicked(2, on, &mut ui, &app);
    assert_eq!(press(Key::Char('c'), CTRL, &mut ui, &app), copied("two"));
}

/// Ctrl and a press on the run that says `said`, at its middle.
fn followed(said: &str) -> (Vec<Command>, crate::Ui) {
    let app = talking(vec![Heard::Said(said.into())]);
    let mut ui = chat_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let run = hits.read().iter().find(|one| one.text.contains(said));
    let rect = run.expect("the words").rect;
    let at = Input::Press {
        x: rect.x + rect.w / 2.0,
        y: rect.y + rect.h / 2.0,
        mods: CTRL,
    };
    (handle(at, &mut ui, &app, &hits, window()), ui)
}

#[test]
fn ctrl_and_a_click_follow_a_link_or_open_a_file_at_its_line() {
    let url = "https://example.com/docs";
    let browse = task::Command::Browse { url: url.into() };
    assert_eq!(followed(url).0, vec![Command::Task(browse)]);
    let (acted, ui) = followed("src/lib.rs:12");
    let at = Some(Selection::at(Caret::new(11, 0)));
    let open = workspace::Command::OpenFile {
        path: "src/lib.rs".into(),
        at,
    };
    assert_eq!(acted, vec![Command::Workspace(open)]);
    assert_eq!(ui.session.tab, crate::views::session::Tab::Files);
}

#[test]
fn while_the_agent_works_past_its_last_record_a_turning_glance_says_so() {
    let mut app = talking(vec![Heard::Said("on it".into())]);
    app.agent.agents[0].1.activity.status = AgentStatus::Working;
    let (frame, _) = crate::view(&app, &chat_ui(), window(), &mut Fonts::embedded());
    assert!(all_texts(&frame).iter().any(|one| one == "thought"));
}
