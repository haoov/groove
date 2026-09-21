//! A note drawn in the surface, under the line it was left on.

use groove_types::{Anchor, Note, NoteOrigin, Said, Timestamp};

use super::*;

/// One note on `line` of the open file, said by `author`.
fn note(line: u32, author: &str, body: &str) -> Note {
    Note {
        origin: NoteOrigin::Thread("t1".into()),
        anchor: Some(Anchor::line("src/lib.rs", line)),
        resolved: false,
        said: vec![said(author, body)],
    }
}

fn said(author: &str, body: &str) -> Said {
    Said {
        author: author.to_string(),
        body: body.to_string(),
        at: Timestamp::new(0),
    }
}

/// The app with those notes on the open file.
fn noted(notes: Vec<Note>) -> AppState {
    let mut app = opened();
    app.workspace.notes = notes;
    app
}

fn in_editor(app: &AppState) -> Vec<String> {
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    row_texts(app, &ui)
}

#[test]
fn a_note_draws_under_the_line_it_was_left_on() {
    let app = noted(vec![note(1, "reviewer", "issue: this leaks")]);
    let drawn = in_editor(&app);
    let at = |text: &str| drawn.iter().position(|one| one == text);
    let line = at("fn TWO() {}").or_else(|| at("TWO")).expect("the line");
    let words = at("issue: this leaks").expect("the note");
    assert!(words > line, "the note stands under its line: {drawn:?}");
    assert_eq!(at("reviewer").map(|one| one < words), Some(true));
}

#[test]
fn a_note_takes_a_row_of_the_surface() {
    let bare = opened();
    let app = noted(vec![note(1, "reviewer", "issue: this leaks")]);
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui),
        crate::views::session::diff::rows_of(&bare, &ui) + 1
    );
}

#[test]
fn a_thread_takes_a_row_for_every_note_of_it() {
    let mut one = note(1, "reviewer", "issue: this leaks");
    one.said.push(said("haoov", "fixed"));
    let bare = opened();
    let app = noted(vec![one]);
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui),
        crate::views::session::diff::rows_of(&bare, &ui) + 2
    );
    let drawn = in_editor(&app);
    assert!(drawn.iter().any(|one| one == "fixed"), "{drawn:?}");
}

#[test]
fn the_line_under_a_note_still_draws_after_it() {
    let app = noted(vec![note(0, "reviewer", "issue: this leaks")]);
    let drawn = in_editor(&app);
    let at = |text: &str| drawn.iter().position(|one| one == text);
    let words = at("issue: this leaks").expect("the note");
    let after = at("TWO").expect("the line under it");
    assert!(after > words, "the note pushes the rows down: {drawn:?}");
}

#[test]
fn a_note_on_another_file_draws_nothing_here() {
    let mut one = note(1, "reviewer", "issue: this leaks");
    one.anchor = Some(Anchor::line("other.rs", 1));
    let app = noted(vec![one]);
    let drawn = in_editor(&app);
    assert!(
        !drawn.iter().any(|one| one == "issue: this leaks"),
        "{drawn:?}"
    );
}

#[test]
fn a_note_beyond_the_file_draws_nothing() {
    let app = noted(vec![note(900, "reviewer", "issue: this leaks")]);
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui),
        crate::views::session::diff::rows_of(&opened(), &ui)
    );
}

#[test]
fn a_note_draws_in_the_diff_views_too() {
    let app = noted(vec![note(1, "reviewer", "issue: this leaks")]);
    for view in [DiffView::Inline, DiffView::Split] {
        let drawn = in_view(&app, view);
        assert!(
            drawn.iter().any(|one| one == "issue: this leaks"),
            "{view:?}: {drawn:?}"
        );
    }
}

#[test]
fn a_click_on_a_note_moves_no_caret() {
    let app = noted(vec![note(0, "reviewer", "issue: this leaks")]);
    assert_eq!(
        crate::views::session::diff::line_at(&app, DiffView::Editor, 1),
        None,
        "row 1 is the note left on row 0"
    );
    assert_eq!(
        crate::views::session::diff::line_at(&app, DiffView::Editor, 2),
        Some(("src/lib.rs".to_string(), 1)),
        "the line after it is the file's second"
    );
}
