//! The agents the agent sends: a card in the conversation, a row in the dock while they work, and the agent's own failures.

use groove_gfx::Fonts;
use groove_types::chat::{Heard, Outcome};

use super::{chat_ui, talking};
use crate::hit::Target;
use crate::tests::{all_texts, click, window};

fn sent(id: &str) -> Heard {
    Heard::Subagent {
        id: id.into(),
        kind: "Explore".into(),
        task: format!("Find {id}"),
    }
}

fn answered(id: &str) -> Heard {
    let outcome = Outcome {
        output: Some(format!("{id} is in observability.py")),
        ..Outcome::default()
    };
    Heard::Result {
        id: id.into(),
        outcome,
    }
}

fn said(n: usize) -> Vec<Heard> {
    (0..n)
        .map(|at| Heard::Said(format!("step {at} done")))
        .collect()
}

#[test]
fn a_working_agent_keeps_a_row_in_the_dock_that_opens_its_card_far_up_the_conversation() {
    let mut heard = vec![sent("metrics")];
    heard.extend(said(40));
    let app = talking(heard);
    let mut ui = chat_ui();
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    let row = Target::Subagent("metrics".into());
    let row = hits.rect_of(&row).expect("its row in the dock");
    assert!(
        hits.sent("metrics").is_some_and(|scroll| scroll > 0.0),
        "the card stands far up"
    );
    click(row, &mut ui, &app, &hits);
    assert_eq!(ui.agent.opened, ["metrics"]);
    assert_eq!(Some(ui.agent.scroll), hits.sent("metrics"));
    let fold = hits.rect_of(&Target::FoldAgents).expect("the dock's fold");
    click(fold, &mut ui, &app, &hits);
    let (_, hits) = crate::view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        hits.rect_of(&Target::Subagent("metrics".into())).is_none(),
        "folded to its head"
    );
}

#[test]
fn a_finished_agent_leaves_the_dock_once_the_user_speaks_again() {
    let ended = talking(vec![sent("metrics"), answered("metrics")]);
    let ui = chat_ui();
    let (frame, hits) = crate::view(&ended, &ui, window(), &mut Fonts::embedded());
    assert!(
        hits.rect_of(&Target::Subagent("metrics".into())).is_some(),
        "done, until the next message"
    );
    assert!(
        all_texts(&frame)
            .iter()
            .any(|one| one.contains("metrics is in observability.py"))
    );
    let spoke = talking(vec![
        sent("metrics"),
        answered("metrics"),
        Heard::You("next".into()),
    ]);
    let (_, hits) = crate::view(&spoke, &ui, window(), &mut Fonts::embedded());
    assert!(hits.rect_of(&Target::Subagent("metrics".into())).is_none());
}

#[test]
fn what_went_wrong_shows_in_the_conversation() {
    let app = talking(vec![Heard::Failed(
        "The agent ended with code 1:\nnot logged in".into(),
    )]);
    let (frame, _) = crate::view(&app, &chat_ui(), window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    assert!(
        texts.iter().any(|one| one.contains("not logged in")),
        "{texts:?}"
    );
}

#[test]
fn a_long_line_of_code_the_agent_shows_wraps_instead_of_running_out_of_sight() {
    let line = format!(
        "kubectl get pods {}TAIL",
        "--selector app=pricing ".repeat(12)
    );
    let app = talking(vec![Heard::Said(format!("Run this:\n\n```\n{line}\n```"))]);
    let (frame, _) = crate::view(&app, &chat_ui(), window(), &mut Fonts::embedded());
    let texts = all_texts(&frame);
    assert!(
        texts.iter().any(|one| one.starts_with("kubectl get pods")),
        "{texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|one| one.ends_with("TAIL") && !one.starts_with("kubectl"))
    );
}
