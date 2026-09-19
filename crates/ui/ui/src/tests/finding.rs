//! Finding text in the surface: the bar, its matches, and where they take you.

use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::DiffView;

use crate::input::{Key, Modifiers};
use crate::tests::{changed_files, full_app, press, shows, window};
use crate::tokens::{ABOVE_MATCH, Tokens};
use crate::views::session::Tab;
use crate::{Focus, Ui, view};

const FILES: [(&str, &str, &str); 2] = [
    ("src/a.rs", "let one = 1;\n", "let one = 11;\n"),
    ("src/b.rs", "let two = 2;\n", "let one = 22;\n"),
];

fn ctrl() -> Modifiers {
    Modifiers {
        ctrl: true,
        ..Modifiers::default()
    }
}

fn typed(text: &str, ui: &mut Ui, app: &AppState) {
    for c in text.chars() {
        press(Key::Char(c), Modifiers::default(), ui, app);
    }
}

fn app() -> AppState {
    let mut app = full_app();
    changed_files(&mut app, &FILES);
    app
}

fn on_code() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    ui.focus = Focus::Workspace;
    ui
}

#[test]
fn the_bar_opens_on_the_chord_and_finds_as_it_is_typed() {
    let app = app();
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    let find = ui.session.find.as_ref().expect("the bar stands");
    assert!(find.typing, "the keyboard is in it");
    assert_eq!(find.count(), "", "nothing is typed yet");

    typed("one", &mut ui, &app);
    let find = ui.session.find.as_ref().expect("still there");
    assert_eq!(
        find.count(),
        "1 / 3",
        "both sides of a.rs and b.rs's new line"
    );
    assert_eq!(find.here().map(|hit| hit.path.as_str()), Some("src/a.rs"));
}

#[test]
fn enter_hands_the_keyboard_back_and_the_bar_stays() {
    let app = app();
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("one", &mut ui, &app);
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    let find = ui.session.find.as_ref().expect("the bar stays up");
    assert!(!find.typing, "typing goes to the code now");
    assert_eq!(find.query.text(), "one", "and it keeps what was searched");
}

#[test]
fn the_chords_step_the_matches_and_wrap() {
    let app = app();
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("one", &mut ui, &app);
    let at = |ui: &Ui| ui.session.find.as_ref().expect("the bar").at;

    press(Key::Char('n'), ctrl(), &mut ui, &app);
    assert_eq!(at(&ui), 1);
    press(Key::Char('n'), ctrl(), &mut ui, &app);
    assert_eq!(at(&ui), 2);
    press(Key::Char('n'), ctrl(), &mut ui, &app);
    assert_eq!(at(&ui), 0, "the last match wraps to the first");
    press(Key::Char('p'), ctrl(), &mut ui, &app);
    assert_eq!(at(&ui), 2, "and back the other way");
}

#[test]
fn escape_takes_the_bar_and_its_marks_away() {
    let app = app();
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("one", &mut ui, &app);
    press(Key::Escape, Modifiers::default(), &mut ui, &app);
    assert!(ui.session.find.is_none());
}

#[test]
fn a_match_in_another_file_opens_it_there() {
    let mut app = app();
    shows(&mut app, FILES[0].0, FILES[0].1, FILES[0].2);
    changed_files(&mut app, &FILES);
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("22", &mut ui, &app);
    let commands = press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert_eq!(commands[0].id(), "workspace.open_file");
}

#[test]
fn a_match_on_a_line_that_went_takes_no_caret() {
    let mut app = app();
    shows(&mut app, FILES[0].0, FILES[0].1, FILES[0].2);
    changed_files(&mut app, &FILES);
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("two", &mut ui, &app);
    let find = ui.session.find.as_ref().expect("the bar");
    assert_eq!(
        find.here().map(|hit| hit.line),
        Some(None),
        "a removed line"
    );
    let commands = press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert!(
        commands.is_empty(),
        "it is shown and marked, but the caret belongs to the new side"
    );
}

#[test]
fn a_match_in_the_open_file_is_held_by_the_caret() {
    let mut app = app();
    shows(&mut app, FILES[0].0, FILES[0].1, FILES[0].2);
    changed_files(&mut app, &FILES);
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("one", &mut ui, &app);
    let commands = press(Key::Char('n'), ctrl(), &mut ui, &app);
    let ids: Vec<&str> = commands.iter().map(|command| command.id()).collect();
    assert_eq!(
        ids,
        ["workspace.edit", "workspace.edit"],
        "the caret moves to the line that came, then holds the match"
    );
}

#[test]
fn the_file_view_searches_only_the_file_it_shows() {
    let mut app = app();
    shows(&mut app, FILES[0].0, FILES[0].1, FILES[0].2);
    changed_files(&mut app, &FILES);
    let mut ui = on_code();
    ui.session.view = DiffView::File;
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("one", &mut ui, &app);
    let find = ui.session.find.as_ref().expect("the bar");
    assert_eq!(
        find.count(),
        "1 / 1",
        "one line of one file, not the change"
    );
    assert_eq!(find.here().map(|hit| hit.line), Some(Some(0)));
}

#[test]
fn the_surface_stands_on_the_match_with_rows_above_it() {
    let mut app = full_app();
    let before: String = (0..60)
        .map(|at| format!("let value_{at} = {at};\n"))
        .collect();
    let after = before.replace("value_40 = 40", "value_40 = 41");
    shows(&mut app, "src/lib.rs", &before, &after);
    let mut ui = on_code();
    ui.session.view = DiffView::File;
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("value_40", &mut ui, &app);
    let line = Tokens::new(1.0).line;
    assert_eq!(
        ui.session.diff,
        (40 - ABOVE_MATCH) as f32 * line,
        "the match is a few rows down, not at the very top"
    );
}

#[test]
fn the_match_it_stands_on_reads_as_a_selection_even_on_a_line_that_went() {
    let mut app = app();
    shows(&mut app, FILES[0].0, FILES[0].1, FILES[0].2);
    changed_files(&mut app, &FILES);
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("two", &mut ui, &app);
    let find = ui.session.find.as_ref().expect("the bar");
    assert_eq!(
        find.here().map(|hit| hit.line),
        Some(None),
        "a removed line"
    );

    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let held = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.held())
        .count();
    assert_eq!(held, 1, "the one it stands on, though no caret can hold it");
}

#[test]
fn split_marks_a_match_on_the_side_that_shows_it_and_not_the_other() {
    let mut app = app();
    shows(&mut app, FILES[0].0, FILES[0].1, FILES[0].2);
    changed_files(&mut app, &FILES);
    let mut ui = on_code();
    ui.session.view = DiffView::Split;
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("= 11", &mut ui, &app);
    let find = ui.session.find.as_ref().expect("the bar");
    assert_eq!(find.count(), "1 / 1", "only the line that came holds it");

    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let rows = hits.rect_of(&crate::hit::Target::Code).expect("the rows");
    let marks: Vec<f32> = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.held() || quad.color == styles.found())
        .map(|quad| quad.rect.x)
        .collect();
    assert!(!marks.is_empty(), "the new side is marked");
    assert!(
        marks.iter().all(|x| *x >= rows.x),
        "and nothing is marked on the old side beside it: {marks:?} against {rows:?}"
    );
}

#[test]
fn only_the_bar_shows_a_caret_while_it_has_the_keyboard() {
    let mut app = app();
    shows(&mut app, FILES[0].0, FILES[0].1, FILES[0].2);
    changed_files(&mut app, &FILES);
    let mut ui = on_code();
    ui.session.view = DiffView::File;
    let carets = |ui: &Ui| {
        let (frame, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
        let rows = hits.rect_of(&crate::hit::Target::Code).expect("the rows");
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.caret())
            .filter(|quad| rows.contains(quad.rect.x, quad.rect.y))
            .count()
    };
    assert_eq!(carets(&ui), 1, "the code carries it at rest");

    press(Key::Char('f'), ctrl(), &mut ui, &app);
    assert_eq!(
        carets(&ui),
        0,
        "and gives it up while the bar is typed into"
    );

    typed("one", &mut ui, &app);
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert_eq!(carets(&ui), 1, "enter hands it back");
}

#[test]
fn the_bar_pops_over_the_rows_on_a_ground_of_its_own() {
    let app = app();
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let above = frame.layers().last().expect("a layer over the rows");
    assert!(
        above.quads.iter().any(|quad| quad.color == styles.action()),
        "the bar stands on its own ground"
    );
    for other in [styles.raised(), styles.panel(), styles.inner()] {
        assert_ne!(
            styles.action(),
            other,
            "and that ground is not a header's or a band's"
        );
    }
}

#[test]
fn every_match_is_marked_under_the_text() {
    let mut app = app();
    shows(&mut app, FILES[0].0, FILES[0].1, FILES[0].2);
    changed_files(&mut app, &FILES);
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("one", &mut ui, &app);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let marked = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.found())
        .count();
    assert_eq!(marked, 3, "one under every match the change holds");
}
