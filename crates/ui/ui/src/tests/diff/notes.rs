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
    app.delivery.shown = notes;
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
        crate::views::session::diff::rows_of(&bare, &ui) + 2,
        "the note and the row of buttons under it"
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
        crate::views::session::diff::rows_of(&bare, &ui) + 3,
        "a row for each note of it, and the buttons under them"
    );
    let drawn = in_editor(&app);
    assert!(drawn.iter().any(|one| one == "fixed"), "{drawn:?}");
}

#[test]
fn a_note_pushes_the_lines_under_it_down() {
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
            drawn.iter().any(|one| one.starts_with("issue: this l")),
            "{view:?}: {drawn:?}"
        );
    }
}

#[test]
fn a_click_on_a_note_moves_no_caret() {
    let app = noted(vec![note(0, "reviewer", "issue: this leaks")]);
    assert_eq!(
        crate::views::session::diff::line_at(&app, &on_diff(), DiffView::Editor, 1),
        None,
        "row 1 is the note left on row 0"
    );
    assert_eq!(
        crate::views::session::diff::line_at(&app, &on_diff(), DiffView::Editor, 2),
        None,
        "row 2 is what the note offers"
    );
    assert_eq!(
        crate::views::session::diff::line_at(&app, &on_diff(), DiffView::Editor, 3),
        Some(("src/lib.rs".to_string(), 1)),
        "the line after it is the file's second"
    );
}

/// The note glyphs the sidebar drew on its file rows.
fn marks_in_sidebar(app: &AppState) -> usize {
    let ui = on_diff();
    let (frame, _) = view(app, &ui, window(), &mut Fonts::embedded());
    let sidebar = crate::layout::Layout::of(window(), &ui).sidebar;
    let shape = groove_ui_kit::base::mark::Mark::Note.shape();
    frame
        .layers()
        .iter()
        .flat_map(|layer| layer.icons.iter())
        .filter(|icon| icon.icon == shape)
        .filter(|icon| icon.rect.x >= sidebar.x && icon.rect.x < sidebar.right())
        .count()
}

#[test]
fn a_file_carrying_a_note_is_marked_in_the_sidebar() {
    let bare = opened();
    assert_eq!(marks_in_sidebar(&bare), 0, "no note, no mark");
    let app = noted(vec![note(1, "reviewer", "issue: this leaks")]);
    assert_eq!(marks_in_sidebar(&app), 1, "the file that carries it");
}

/// A note of this session's own, which its row of buttons can act on.
fn own(line: u32, body: &str) -> Note {
    Note {
        origin: NoteOrigin::Local(groove_types::AnnotationId::new("n1")),
        anchor: Some(Anchor::line("src/lib.rs", line)),
        resolved: false,
        said: vec![said("you", body)],
    }
}

/// Where the note's own marks stand in the surface.
fn gutter_marks(app: &AppState) -> Vec<f32> {
    let ui = on_diff();
    let (frame, hits) = view(app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let shape = groove_ui_kit::base::mark::Mark::Note.shape();
    frame
        .layers()
        .iter()
        .flat_map(|layer| layer.icons.iter())
        .filter(|icon| icon.icon == shape && code.contains(icon.rect.x, icon.rect.y))
        .map(|icon| icon.rect.x)
        .collect()
}

#[test]
fn the_notes_mark_ends_where_a_line_number_ends() {
    let app = noted(vec![own(1, "issue: this leaks")]);
    let ui = on_diff();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let numbers: Vec<f32> = frame.layers()[0]
        .texts
        .iter()
        .filter(|run| code.contains(run.x, run.y) && run.text.parse::<u32>().is_ok())
        .map(|run| run.x + width_of(&run.text, run.style))
        .collect();
    let ends = numbers.first().copied().expect("a line number");
    let mark = gutter_marks(&app).first().copied().expect("the mark");
    let size = Tokens::new(1.0).small;
    assert!(
        (mark + size - ends).abs() < 1.0,
        "the mark ends where the numbers do: {} against {ends}",
        mark + size
    );
}

/// How wide a text run stands, measured as the frame measured it.
fn width_of(text: &str, style: groove_gfx::TextStyle) -> f32 {
    let mut fonts = Fonts::embedded();
    fonts.measure(text, style.font, style.weight, style.size)
}

#[test]
fn what_a_note_says_starts_where_the_code_does() {
    let app = noted(vec![own(1, "issue: this leaks")]);
    let ui = on_diff();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let words = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text == "issue: this leaks" && code.contains(run.x, run.y))
        .map(|run| run.x)
        .expect("the words");
    assert_eq!(words, hits.chars().left, "on the code's own column");
}

#[test]
fn the_buttons_start_where_the_words_do() {
    let app = noted(vec![own(1, "issue: this leaks")]);
    let ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let id = groove_types::NoteOrigin::Local(groove_types::AnnotationId::new("n1"));
    let first = hits
        .rect_of(&Target::Note(id, crate::hit::NoteButton::Edit))
        .expect("the first button");
    assert_eq!(first.x, hits.chars().left, "under the words themselves");
}

#[test]
fn a_line_carrying_a_note_takes_its_own_ground() {
    let grounds = |app: &AppState| {
        let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
        let ui = on_diff();
        let (frame, _) = view(app, &ui, window(), &mut Fonts::embedded());
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.noted())
            .count()
    };
    assert_eq!(grounds(&opened()), 0, "no note, no ground");
    assert_eq!(
        grounds(&noted(vec![own(1, "issue")])),
        1,
        "the line it is on"
    );
}

#[test]
fn a_note_of_this_session_offers_what_to_do_with_it() {
    let app = noted(vec![own(1, "issue: this leaks")]);
    let ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let id = groove_types::NoteOrigin::Local(groove_types::AnnotationId::new("n1"));
    for button in [
        crate::hit::NoteButton::Edit,
        crate::hit::NoteButton::Resolve,
        crate::hit::NoteButton::Delete,
    ] {
        assert!(
            hits.rect_of(&Target::Note(id.clone(), button)).is_some(),
            "{button:?} stands under the note"
        );
    }
    assert!(
        hits.rect_of(&Target::Note(id, crate::hit::NoteButton::Post))
            .is_none(),
        "posting waits for the forge"
    );
}

#[test]
fn a_thread_offers_what_the_forge_answers_for_it() {
    let app = noted(vec![note(1, "reviewer", "issue: this leaks")]);
    let ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let origin = NoteOrigin::Thread("t1".into());
    assert!(
        hits.rect_of(&Target::Note(origin.clone(), crate::hit::NoteButton::Reply))
            .is_some(),
        "a thread takes a reply"
    );
    assert!(
        hits.rect_of(&Target::Note(origin, crate::hit::NoteButton::Delete))
            .is_none(),
        "and none of this session's own writes"
    );
}

#[test]
fn a_button_under_the_pointer_takes_the_acted_ground() {
    let app = noted(vec![own(1, "issue: this leaks")]);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let grounds = |ui: &Ui| {
        let (frame, _) = view(&app, ui, window(), &mut Fonts::embedded());
        frame
            .layers()
            .iter()
            .flat_map(|layer| layer.quads.iter())
            .filter(|quad| quad.color == styles.action())
            .count()
    };
    let ui = on_diff();
    assert_eq!(grounds(&ui), 0, "nothing is under the pointer");
    let mut on_it = on_diff();
    on_it.hover = Some(Target::Note(
        groove_types::NoteOrigin::Local(groove_types::AnnotationId::new("n1")),
        crate::hit::NoteButton::Edit,
    ));
    assert_eq!(grounds(&on_it), 1, "the one it stands on");
}

#[test]
fn a_note_says_which_lines_it_is_about() {
    let mut one = own(1, "issue: this leaks");
    one.anchor = Some(Anchor {
        path: "src/lib.rs".into(),
        start_line: 0,
        end_line: 2,
    });
    let app = noted(vec![one]);
    let drawn = in_editor(&app);
    assert!(
        drawn.iter().any(|text| text == "1-3"),
        "the lines a file numbers them: {drawn:?}"
    );
}

#[test]
fn a_note_on_one_line_says_that_line() {
    let app = noted(vec![own(1, "issue: this leaks")]);
    let drawn = in_editor(&app);
    assert!(drawn.iter().any(|text| text == "2"), "{drawn:?}");
}

#[test]
fn only_the_row_that_opens_a_note_says_its_lines() {
    let mut one = note(1, "reviewer", "issue: this leaks");
    one.said.push(said("haoov", "fixed"));
    let app = noted(vec![one]);
    let drawn = in_editor(&app);
    assert_eq!(
        drawn.iter().filter(|text| *text == "2").count(),
        2,
        "the line's own number, and the note's once: {drawn:?}"
    );
}

#[test]
fn a_commit_shows_none_of_the_sessions_notes() {
    let mut app = noted(vec![own(1, "issue: this leaks")]);
    let ui = on_diff();
    app.workspace.commit = Some(groove_types::CommitEntry {
        sha: "abc".into(),
        short_sha: "abc".into(),
        message: "feat: one".into(),
        author: "T".into(),
        at: Timestamp::new(0),
        is_base: false,
    });
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui),
        crate::views::session::diff::rows_of(&opened(), &ui),
        "a note stands on the working tree's lines, not a commit's"
    );
    let drawn = in_editor(&app);
    assert!(
        !drawn.iter().any(|one| one.contains("this leaks")),
        "{drawn:?}"
    );
}

#[test]
fn a_long_note_wraps_onto_rows_of_its_own_and_loses_no_word() {
    let body = "issue: this leaks the handle every time the file is opened again \
                and nothing ever closes it";
    let app = noted(vec![note(1, "reviewer", body)]);
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    ui.session.note_cols = 24;
    let drawn = row_texts(&app, &ui);
    for word in body.split_whitespace() {
        assert!(
            drawn
                .iter()
                .any(|one| one.split_whitespace().any(|w| w == word)),
            "{word} is drawn: {drawn:?}"
        );
    }
    assert!(
        !drawn.iter().any(|one| one.ends_with('\u{2026}')),
        "nothing is cut: {drawn:?}"
    );
    let rows = drawn
        .iter()
        .filter(|one| body.contains(one.trim()) && !one.trim().is_empty())
        .count();
    assert!(rows >= 4, "it takes the rows it needs: {drawn:?}");
}

#[test]
fn a_note_keeps_the_lines_it_was_written_with() {
    let app = noted(vec![note(1, "reviewer", "first line\nsecond line")]);
    let drawn = in_editor(&app);
    assert!(drawn.iter().any(|one| one == "first line"), "{drawn:?}");
    assert!(drawn.iter().any(|one| one == "second line"), "{drawn:?}");
}
