//! The transcript's blocks: edits gathered in one, the slash offers in their groups, a question
//! stepped back.

use groove_gfx::Fonts;
use groove_types::Timestamp;
use groove_types::chat::{Heard, Outcome, Patch, Slash};

use super::{WORKTREE, chat_ui, prompt_ui, talking};
use crate::hit::{Answering, Target};
use crate::input::{Key, Modifiers};
use crate::tests::{all_texts, click, press, typed, window};

fn edited(id: &str, path: &str) -> [Heard; 2] {
    let patch = Patch {
        added: 3,
        removed: 1,
        line: 4,
    };
    let outcome = Outcome {
        patch: Some(patch),
        ..Outcome::default()
    };
    [
        Heard::Edit {
            id: id.into(),
            path: format!("{WORKTREE}/{path}"),
            created: false,
        },
        Heard::Result {
            id: id.into(),
            outcome,
        },
    ]
}

#[test]
fn edits_back_to_back_stand_in_one_block_each_file_a_link() {
    let mut heard = Vec::new();
    heard.extend(edited("a", "src/one.rs"));
    heard.extend(edited("b", "src/two.rs"));
    let app = talking(heard);
    let (frame, hits) = crate::view(&app, &chat_ui(), window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    assert!(texts.iter().any(|one| one == "EDITED · 2 FILES"), "{texts:?}");
    assert!(texts.iter().any(|one| one == "+6"), "the block's sum: {texts:?}");
    for path in ["src/one.rs", "src/two.rs"] {
        let link = Target::Edited {
            path: format!("{WORKTREE}/{path}"),
            line: 4,
        };
        assert!(hits.rect_of(&link).is_some(), "{path} links to its hunk");
    }
}

#[test]
fn the_agent_speaks_under_its_name_once_a_turn() {
    let app = talking(vec![
        Heard::Clock(Timestamp::new(100)),
        Heard::You("go".into()),
        Heard::Said("first".into()),
        Heard::Said("second".into()),
    ]);
    let (frame, _) = crate::view(&app, &chat_ui(), window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    let named = texts.iter().filter(|one| *one == "AGENT").count();
    assert_eq!(named, 1, "{texts:?}");
    assert!(texts.iter().any(|one| one == "YOU"));
}

#[test]
fn the_slash_offers_the_agent_s_commands_then_groove_s_under_their_heads() {
    let mut app = talking(Vec::new());
    let slash = |name: &str| Slash {
        name: name.into(),
        about: format!("runs {name}"),
    };
    let listed = ["review", "groove:review-mr"].map(slash);
    app.agent.agents[0].1.commands = listed.to_vec();
    app.skill.skills = vec![crate::tests::agent::skill("review-mr")];
    let mut ui = prompt_ui();
    typed("/rev", &mut ui, &app);
    let (offers, own) = crate::views::session::agent_pane::offered::grouped(&app, &ui);
    let names: Vec<&str> = offers.iter().map(|one| one.label).collect();
    assert_eq!((names, own), (vec!["review", "groove:review-mr"], 1));
    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    for head in ["THE AGENT · 1", "GROOVE · 1"] {
        assert!(texts.iter().any(|one| one == head), "{head}: {texts:?}");
    }
}

#[test]
fn back_undoes_the_last_pick_of_a_question() {
    let mut app = talking(Vec::new());
    let two = [&["Tea", "Coffee"][..], &["Now", "Later"][..]];
    super::asked::asked(&mut app, two.map(super::asked::question).to_vec());
    let mut ui = chat_ui();
    press(Key::Char('2'), Modifiers::default(), &mut ui, &app);
    assert_eq!(ui.agent.picked("r1"), ["Coffee"]);
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let back = hits.rect_of(&Target::Answer(Answering::Back)).expect("back");
    assert!(click(back, &mut ui, &app, &hits).is_empty());
    assert!(ui.agent.picked("r1").is_empty(), "the first question again");
}

#[test]
fn with_the_rail_folded_the_conversation_stays_clear_of_its_headers() {
    let said = (0..60).map(|at| Heard::Said(format!("step {at} done")));
    let app = talking(said.collect());
    let mut ui = chat_ui();
    (ui.rail.stowed, ui.rail.folded) = (true, true);
    let (frame, _) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let pane = crate::layout::Layout::of(window(), &ui).agent;
    let texts = frame.layers().iter().flat_map(|one| one.texts.iter());
    let steps = texts.filter(|run| run.text.starts_with("step "));
    let outside = steps.filter(|run| run.clip.y < pane.y);
    assert_eq!(outside.count(), 0, "no line reaches the bar room over the pane");
}
