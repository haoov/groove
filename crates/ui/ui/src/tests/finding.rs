//! Finding text in the surface: the bar, its matches, and where they take you.

use groove_controllers::{AppState, Command, workspace};
use groove_gfx::Fonts;
use groove_types::DiffView;

use crate::input::{Key, Modifiers};
use crate::tests::{changed_files, full_app, press, shows, window};
use crate::views::session::{Tab, Term};
use crate::{Focus, Ui, view};
use groove_ui_kit::base::tokens::Tokens;

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
    ui.session.tab = crate::views::session::Tab::Files;
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
fn the_match_stands_in_the_middle_of_the_rows_at_the_window_s_scale() {
    let mut app = full_app();
    let before: String = (0..120)
        .map(|at| format!("let value_{at} = {at};\n"))
        .collect();
    let after = before.replace("value_60 = 60", "value_60 = 61");
    shows(&mut app, "src/lib.rs", &before, &after);
    let mut ui = on_code();
    ui.session.tab = crate::views::session::Tab::Files;
    let window = crate::tests::metrics(2560, 1600, 2.0);
    let (_, hits) = view(&app, &ui, window, &mut Fonts::embedded());
    let keys = std::iter::once((Key::Char('f'), ctrl())).chain(
        "value_60"
            .chars()
            .map(|c| (Key::Char(c), Modifiers::default())),
    );
    for (key, mods) in keys {
        let input = crate::input::Input::Key { key, mods };
        crate::input::handle(input, &mut ui, &app, &hits, window);
    }
    let line = window.tokens().line;
    let rows = hits.rect_of(&crate::hit::Target::Code).expect("the rows");
    let middle = 60.5 * line - ui.session.scroll();
    assert!(
        (middle - rows.h / 2.0).abs() < line,
        "the match sits at {middle} of rows {} tall",
        rows.h
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
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let held = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.found())
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
    assert_eq!(ui.session.find.as_ref().expect("the bar").count(), "1 / 1");
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let rows = hits.rect_of(&crate::hit::Target::Code).expect("the rows");
    let rail = crate::layout::Layout::of(window(), &ui).rail.right();
    let mark = |quad: &&groove_gfx::Quad| [styles.held(), styles.found()].contains(&quad.color);
    let marks: Vec<f32> = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.rect.x >= rail && mark(quad))
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
    ui.session.tab = crate::views::session::Tab::Files;
    let carets = |ui: &Ui| {
        let (frame, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
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
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let above = frame.layers().last().expect("a layer over the rows");
    assert!(
        above.quads.iter().any(|quad| quad.color == styles.action()),
        "the bar stands on its own ground"
    );
    for other in [styles.raised(), styles.band(), styles.deep()] {
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
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let quads = &frame.layers()[0].quads;
    let count = |color| quads.iter().filter(|quad| quad.color == color).count();
    assert_eq!(
        count(styles.found()),
        3,
        "one under every match the change holds"
    );
    assert_eq!(
        count(styles.standing()),
        1,
        "and peach over the one it stands on"
    );
}

#[test]
fn the_chord_opens_the_bar_on_the_worktree_and_typing_searches_it() {
    let app = app();
    let mut ui = on_code();
    press(Key::Char('f'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    assert_eq!(
        ui.session.bar.typing,
        Some(Term::Text),
        "the keyboard is in the text term"
    );

    let commands = press(Key::Char('x'), Modifiers::default(), &mut ui, &app);
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert_eq!(commands[0].id(), "workspace.grep", "as it is typed");
}

#[test]
fn the_two_terms_narrow_together_whichever_is_typed_first() {
    let app = app();
    let mut ui = on_code();
    press(Key::Char('p'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    typed("ui", &mut ui, &app);
    assert_eq!(ui.session.bar.path.text(), "ui");

    press(Key::Char('f'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    assert_eq!(
        ui.session.bar.path.text(),
        "ui",
        "the path is kept as scope"
    );
    let commands = typed_last("Aligned", &mut ui, &app);
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Grep {
            query: "Aligned".into(),
            under: "ui".into(),
        })],
        "the text is looked for only under the path"
    );

    press(Key::Char('p'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    assert_eq!(
        ui.session.bar.text.text(),
        "Aligned",
        "and now the other way"
    );
    let commands = typed_last("src", &mut ui, &app);
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Grep {
            query: "Aligned".into(),
            under: "src".into(),
        })],
        "a path typed over what was found searches again under it"
    );
}

/// What the last keystroke of `text` asked for.
fn typed_last(text: &str, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let mut commands = Vec::new();
    for c in text.chars() {
        commands = press(Key::Char(c), Modifiers::default(), ui, app);
    }
    commands
}

#[test]
fn a_found_line_opens_its_file_where_it_sits() {
    let mut app = app();
    app.workspace.found = vec![groove_controllers::workspace_service::Found {
        path: "src/b.rs".into(),
        line: 3,
        text: "let one = 22;".into(),
        at: (4, 7),
    }];
    let mut ui = on_code();
    press(Key::Char('f'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    press(Key::Char('o'), Modifiers::default(), &mut ui, &app);

    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&crate::hit::Target::Found(0))
        .expect("the sidebar lists what was found");
    let commands = crate::tests::click(row, &mut ui, &app, &hits);
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::OpenFile {
            path: "src/b.rs".into(),
            at: Some(groove_types::Selection {
                anchor: groove_types::Caret::new(3, 4),
                head: groove_types::Caret::new(3, 7),
            }),
        })],
        "the file, at the line, holding the match"
    );
}

#[test]
fn the_bar_stands_only_once_a_search_asks_for_it() {
    let app = app();
    let mut ui = crate::tests::sidebar_ui();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for term in [Term::Path, Term::Text] {
        assert!(
            hits.rect_of(&crate::hit::Target::Term(term)).is_none(),
            "{term:?} waits to be asked for"
        );
    }

    press(Key::Char('f'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    assert_eq!(
        ui.session.bar.typing,
        Some(Term::Text),
        "the chord opens it"
    );
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    for term in [Term::Path, Term::Text] {
        let row = hits
            .rect_of(&crate::hit::Target::Term(term))
            .unwrap_or_else(|| panic!("{term:?} stands in the bar"));
        assert!(crate::tests::click(row, &mut ui, &app, &hits).is_empty());
        assert_eq!(ui.session.bar.typing, Some(term), "the click focuses it");
    }
}

#[test]
fn a_click_on_a_term_keeps_what_it_holds() {
    let app = app();
    let mut ui = crate::tests::sidebar_ui();
    press(Key::Char('p'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    typed("src", &mut ui, &app);
    press(Key::Escape, Modifiers::default(), &mut ui, &app);
    press(Key::Char('p'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    typed("ui", &mut ui, &app);
    press(Key::Enter, Modifiers::default(), &mut ui, &app);

    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&crate::hit::Target::Term(Term::Path))
        .expect("the path term");
    crate::tests::click(row, &mut ui, &app, &hits);
    assert_eq!(
        ui.session.bar.path.text(),
        "ui",
        "a click does not spend it"
    );
    assert_eq!(ui.session.bar.typing, Some(Term::Path));
}

#[test]
fn a_click_on_the_find_bar_takes_the_keyboard_back() {
    let app = app();
    let mut ui = on_code();
    press(Key::Char('f'), ctrl(), &mut ui, &app);
    typed("one", &mut ui, &app);
    press(Key::Enter, Modifiers::default(), &mut ui, &app);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let bar = hits
        .rect_of(&crate::hit::Target::Finding)
        .expect("the bar over the rows");
    crate::tests::click(bar, &mut ui, &app, &hits);
    let find = ui.session.find.as_ref().expect("still there");
    assert!(find.typing, "the keyboard is in it again");
    assert_eq!(find.query.text(), "one", "with what it held");
}
