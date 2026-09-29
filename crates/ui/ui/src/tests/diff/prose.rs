//! What a note in the surface says, drawn as the Markdown it is written in.

use groove_gfx::{Font, Fonts, Weight};
use groove_types::{Anchor, Note, NoteOrigin, Said, Timestamp};

use super::*;

fn noted(body: &str) -> AppState {
    let mut app = opened();
    app.delivery.shown = vec![Note {
        origin: NoteOrigin::Thread("t1".into()),
        anchor: Some(Anchor::line("src/lib.rs", 1)),
        resolved: false,
        said: vec![Said {
            author: "reviewer".into(),
            body: body.into(),
            at: Timestamp::new(0),
        }],
    }];
    app
}

#[test]
fn a_note_draws_its_emphasis_code_and_links_without_their_marks() {
    let app =
        noted("**issue (blocking):** use `drop` [here](https://example.com/a)\n\n- [ ] then this");
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let runs = &frame.layers()[0].texts;
    let run = |text: &str| {
        runs.iter()
            .find(|one| one.text.trim_end() == text)
            .unwrap_or_else(|| {
                panic!(
                    "{text}: {:?}",
                    runs.iter().map(|one| &one.text).collect::<Vec<_>>()
                )
            })
    };
    assert_eq!(run("issue (blocking):").style.weight, Weight::Bold);
    assert_eq!(run("drop").style.font, Font::Mono);
    run("then this");
    assert!(
        !runs
            .iter()
            .any(|one| one.text.contains("**") || one.text.contains("[ ]"))
    );
    let link = Target::Link("https://example.com/a".into());
    assert!(hits.rect_of(&link).is_some(), "the link is clickable");
}
