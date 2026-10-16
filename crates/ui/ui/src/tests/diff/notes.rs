//! A note drawn in the surface, under the line it was left on.

use groove_types::{Anchor, Note, NoteOrigin, Said, Timestamp};

use super::*;

mod folds;
mod labels;
mod lines;
mod tiles;

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
    ui.session.tab = crate::views::session::Tab::Files;
    row_texts(app, &ui)
}

#[test]
fn a_note_draws_under_the_line_it_was_left_on() {
    let app = noted(vec![note(1, "reviewer", "issue: this leaks")]);
    let drawn = in_editor(&app);
    let at = |text: &str| drawn.iter().position(|one| one == text);
    let line = at("fn TWO() {}").or_else(|| at("TWO")).expect("the line");
    let words = at("this leaks").expect("the note");
    assert!(words > line, "the note stands under its line: {drawn:?}");
    assert_eq!(at("reviewer").map(|one| one < words), Some(true));
}

#[test]
fn a_note_stands_in_a_card_with_no_room_around_it() {
    let app = noted(vec![note(1, "reviewer", "issue: this leaks")]);
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    let (frame, hits) = view_of(&app, &ui);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let (line, thin) = (Tokens::new(1.0).line, Tokens::new(1.0).hairline);
    let code = hits.rect_of(&crate::hit::Target::Code).expect("the rows");
    let quads = &frame.layers()[0].quads;
    let mut card: Vec<groove_gfx::Rect> = quads
        .iter()
        .filter(|quad| quad.color == styles.band() && code.contains(quad.rect.x, quad.rect.y))
        .map(|quad| quad.rect)
        .collect();
    card.sort_by(|a, b| a.y.total_cmp(&b.y));
    let heights: Vec<f32> = card.iter().map(|rect| rect.h).collect();
    assert_eq!(heights, [line, line, line, line], "{card:?}");
    assert!(card[0].x > code.x && card[0].right() < code.right());
    let edge = |y: f32| {
        quads
            .iter()
            .any(|quad| quad.color == styles.line() && quad.rect.h == thin && quad.rect.y == y)
    };
    assert!(edge(card[0].y), "closed right under the code above");
    assert!(
        edge(card[2].y + line / 2.0),
        "the buttons ruled off from what the note says, half a row from each"
    );
    assert!(
        edge(card[3].bottom() - thin),
        "and right over the code below"
    );
    let deep: Vec<groove_gfx::Rect> = quads
        .iter()
        .filter(|quad| quad.color == styles.deep() && code.contains(quad.rect.x, quad.rect.y))
        .map(|quad| quad.rect)
        .collect();
    let rows = deep.iter().filter(|rect| rect.w > card[0].w).count();
    assert_eq!(
        rows, 4,
        "every row of the note stands on the deepest ground: {deep:?}"
    );
}

#[test]
fn each_comment_opens_on_a_header_that_names_who_from_where_and_its_label() {
    let mut one = note(1, "Marie Dupont", "issue (blocking): this leaks");
    one.said.push(said("haoov", "fixed"));
    let drawn = in_editor(&noted(vec![one]));
    let at = |text: &str| drawn.iter().position(|one| one == text);
    let header = ["MD", "Marie Dupont", "MR", "issue", "blocking"].map(&at);
    assert!(header.iter().all(Option::is_some), "{drawn:?}");
    let words = at("this leaks").expect("the words, their label off");
    assert!(header.iter().all(|one| one.is_some_and(|one| one < words)));
    let reply = at("HA").expect("the reply's own header");
    assert!(at("haoov").is_some_and(|one| one > reply) && at("fixed") > Some(reply));
    assert!(
        !drawn.iter().any(|one| one.contains("issue (blocking)")),
        "{drawn:?}"
    );
    let local = in_editor(&noted(vec![own(1, "thought: maybe")]));
    assert!(local.iter().any(|one| one == "local"), "{local:?}");
    assert!(
        local.iter().any(|one| one == "me") && local.iter().any(|one| one == "ME"),
        "the user's own note reads as theirs: {local:?}"
    );
}

#[test]
fn a_note_takes_a_row_of_the_surface() {
    let bare = opened();
    let app = noted(vec![note(1, "reviewer", "issue: this leaks")]);
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui, drawn_cols(&app, &ui)),
        crate::views::session::diff::rows_of(&bare, &ui, drawn_cols(&bare, &ui)) + 4,
        "its header, its words, a rule, the row of buttons"
    );
}

#[test]
fn a_thread_takes_a_header_and_a_row_for_every_comment_of_it() {
    let mut one = note(1, "reviewer", "issue: this leaks");
    one.said.push(said("haoov", "fixed"));
    let bare = opened();
    let app = noted(vec![one]);
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui, drawn_cols(&app, &ui)),
        crate::views::session::diff::rows_of(&bare, &ui, drawn_cols(&bare, &ui)) + 6,
        "a header and a row for each comment of it, the buttons, room above and below"
    );
    let drawn = in_editor(&app);
    assert!(drawn.iter().any(|one| one == "fixed"), "{drawn:?}");
}

#[test]
fn a_note_pushes_the_lines_under_it_down() {
    let app = noted(vec![note(0, "reviewer", "issue: this leaks")]);
    let drawn = in_editor(&app);
    let at = |text: &str| drawn.iter().position(|one| one == text);
    let words = at("this leaks").expect("the note");
    let after = at("TWO").expect("the line under it");
    assert!(after > words, "the note pushes the rows down: {drawn:?}");
}

#[test]
fn a_note_on_another_file_draws_nothing_here() {
    let mut one = note(1, "reviewer", "issue: this leaks");
    one.anchor = Some(Anchor::line("other.rs", 1));
    let app = noted(vec![one]);
    let drawn = in_editor(&app);
    assert!(!drawn.iter().any(|one| one == "this leaks"), "{drawn:?}");
}

#[test]
fn a_note_beyond_the_file_draws_nothing() {
    let app = noted(vec![note(900, "reviewer", "issue: this leaks")]);
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    assert_eq!(
        crate::views::session::diff::rows_of(&app, &ui, drawn_cols(&app, &ui)),
        crate::views::session::diff::rows_of(&opened(), &ui, drawn_cols(&opened(), &ui))
    );
}

#[test]
fn a_note_draws_in_the_diff_views_too() {
    let app = noted(vec![note(1, "reviewer", "issue: this leaks")]);
    for view in [DiffView::Inline, DiffView::Split] {
        let drawn = in_view(&app, crate::views::session::Face::Stream(view));
        assert!(
            drawn.iter().any(|one| one.starts_with("this l")),
            "{view:?}: {drawn:?}"
        );
    }
}

#[test]
fn a_click_on_a_note_moves_no_caret() {
    let app = noted(vec![note(0, "reviewer", "issue: this leaks")]);
    let line_at = |row| {
        crate::views::session::diff::line_at(
            &app,
            &on_diff(),
            (
                crate::views::session::Face::File,
                drawn_cols(&app, &on_diff()),
            ),
            row,
        )
    };
    for row in 1..=4 {
        assert_eq!(line_at(row), None, "row {row} is the note left on row 0");
    }
    assert_eq!(
        line_at(5),
        Some(("src/lib.rs".to_string(), 1, 0)),
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

#[test]
fn a_note_s_header_starts_where_the_code_does_and_its_words_a_step_in() {
    let app = noted(vec![own(1, "issue: this leaks")]);
    let ui = on_diff();
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let words = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text == "this leaks" && code.contains(run.x, run.y))
        .map(|run| run.x)
        .expect("the words");
    let initials = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text == "ME" && code.contains(run.x, run.y))
        .map(|run| run.x)
        .expect("the initials");
    assert_eq!(
        initials,
        hits.chars().left,
        "the header on the code's own column"
    );
    assert_eq!(
        words,
        hits.chars().left + Tokens::new(1.0).xl,
        "the words a step in, under the author"
    );
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
        hits.rect_of(&Target::Note(id.clone(), crate::hit::NoteButton::Post))
            .is_none(),
        "posting waits for the forge"
    );
    let at = |button| {
        hits.rect_of(&Target::Note(id.clone(), button))
            .expect("drawn")
    };
    let (edit, resolve, delete) = (
        at(crate::hit::NoteButton::Edit),
        at(crate::hit::NoteButton::Resolve),
        at(crate::hit::NoteButton::Delete),
    );
    assert!(edit.x < resolve.x, "edit, then resolve");
    let gap = delete.x - resolve.right();
    assert!(gap > edit.w, "delete stands apart, at the end: {gap}");
}

#[test]
fn a_thread_offers_what_the_forge_answers_for_it() {
    let app = noted(vec![note(1, "reviewer", "issue: this leaks")]);
    let ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let origin = NoteOrigin::Thread("t1".into());
    let field = hits
        .rect_of(&Target::Note(origin.clone(), crate::hit::NoteButton::Reply))
        .expect("a thread takes a reply");
    let resolve = hits
        .rect_of(&Target::Note(
            origin.clone(),
            crate::hit::NoteButton::Resolve,
        ))
        .expect("and is resolved");
    assert!(
        field.w > resolve.w * 4.0,
        "the reply is a field across the row"
    );
    assert!(resolve.x > field.right(), "resolve stands at its end");
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
        crate::views::session::diff::rows_of(&app, &ui, drawn_cols(&app, &ui)),
        crate::views::session::diff::rows_of(&opened(), &ui, drawn_cols(&opened(), &ui)),
        "a note stands on the working tree's lines, not a commit's"
    );
    let drawn = in_editor(&app);
    assert!(
        !drawn.iter().any(|one| one.contains("this leaks")),
        "{drawn:?}"
    );
}
