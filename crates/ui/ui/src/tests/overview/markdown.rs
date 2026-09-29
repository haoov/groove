//! The task body drawn as Markdown: its headings, emphasis, code, lists and links.

use groove_gfx::{Font, TextRun, Weight};

use super::working_a_task;
use crate::hit::Target;
use crate::tests::metrics;
use crate::{Ui, view};

const BODY: &str = "## Goal\n\nClose **the gates** and *then* `ship`.\n\n\
- [x] tests\n- [ ] docs\n\n```\nlet one = 1;\n```\n\nSee [the runbook](https://example.com/run).";

fn drawn() -> (Vec<TextRun>, crate::hit::Hits) {
    let mut app = working_a_task();
    app.task
        .bodies
        .insert("gh-haoov-groove-50".into(), BODY.into());
    let (frame, hits) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    (frame.layers()[0].texts.clone(), hits)
}

fn run<'a>(runs: &'a [TextRun], text: &str) -> &'a TextRun {
    runs.iter()
        .find(|one| one.text.trim_end() == text)
        .unwrap_or_else(|| panic!("{text} is drawn"))
}

#[test]
fn a_heading_stands_larger_than_its_paragraph_and_shows_no_hashes() {
    let (runs, _) = drawn();
    assert!(run(&runs, "Goal").style.size > run(&runs, "Close").style.size);
    assert_eq!(run(&runs, "Goal").style.weight, Weight::SemiBold);
    assert!(!runs.iter().any(|one| one.text.contains('#')));
}

#[test]
fn emphasis_and_code_take_their_own_faces() {
    let (runs, _) = drawn();
    assert_eq!(run(&runs, "the gates").style.weight, Weight::Bold);
    assert_eq!(run(&runs, "then").style.font, Font::Italic);
    assert_eq!(run(&runs, "ship").style.font, Font::Mono);
    assert_eq!(run(&runs, "let one = 1;").style.font, Font::Mono);
    let text = run(&runs, "Close").style.size;
    assert_eq!(
        run(&runs, "ship").style.size,
        text,
        "code as large as its text"
    );
    assert_eq!(run(&runs, "let one = 1;").style.size, text);
    assert!(
        !runs
            .iter()
            .any(|one| one.text.contains("**") || one.text.contains('`'))
    );
}

#[test]
fn a_task_list_draws_its_items_without_their_brackets() {
    let (runs, _) = drawn();
    run(&runs, "tests");
    run(&runs, "docs");
    assert!(
        !runs
            .iter()
            .any(|one| one.text.contains("[x]") || one.text.contains("[ ]"))
    );
}

#[test]
fn a_link_is_a_target_that_opens_its_address() {
    let (runs, hits) = drawn();
    run(&runs, "the runbook");
    let target = Target::Link("https://example.com/run".into());
    assert!(hits.rect_of(&target).is_some(), "the link is clickable");
}
