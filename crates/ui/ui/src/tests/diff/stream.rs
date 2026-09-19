//! The diff tab as one surface over every changed file.

use super::*;
use crate::tests::changed_files;
use groove_controllers::{Command, workspace};
use groove_types::Caret;

const FILES: [(&str, &str, &str); 2] = [
    ("src/a.rs", "one\n", "ONE\n"),
    ("src/b.rs", "two\n", "TWO\n"),
];

fn both() -> AppState {
    let mut app = with_files();
    changed_files(&mut app, &FILES);
    app
}

#[test]
fn every_changed_file_draws_under_a_row_naming_it() {
    let drawn = texts(&both(), &on_diff());
    for (path, before, after) in FILES {
        assert!(drawn.iter().any(|text| text == path), "{path}: {drawn:?}");
        assert!(drawn.iter().any(|text| text == before.trim()));
        assert!(drawn.iter().any(|text| text == after.trim()));
    }
}

#[test]
fn a_click_in_a_file_that_is_not_open_opens_it_where_it_was_clicked() {
    let app = both();
    let mut ui = on_diff();
    let (_, hits) = view_of(&app, &ui);
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let tokens = Tokens::new(1.0);
    let point = (hits.chars().left + 1.0, code.y + tokens.line * 2.0 + 1.0);
    let commands = handle(
        Input::Press {
            x: point.0,
            y: point.1,
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::OpenFile {
            path: "src/a.rs".into(),
            at: Some(Caret::new(0, 0)),
        })],
        "the second row under the head is the first file's new line"
    );
}

#[test]
fn a_file_in_the_sidebar_scrolls_the_surface_to_where_it_starts() {
    let app = both();
    let mut ui = on_diff();
    let (_, hits) = view_of(&app, &ui);
    let row = hits
        .rect_of(&Target::File("src/b.rs".into()))
        .expect("the sidebar lists it");
    click(row, &mut ui, &app, &hits);
    let head = app
        .workspace
        .changes
        .head_of("src/b.rs")
        .expect("the second file");
    assert_eq!(ui.session.diff, head as f32 * Tokens::new(1.0).line);
}

#[test]
fn the_header_names_the_file_the_surface_stands_in() {
    let app = both();
    let mut ui = on_diff();
    let head = app.workspace.changes.head_of("src/b.rs").expect("the file");
    ui.session.diff = head as f32 * Tokens::new(1.0).line;
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|text| text.ends_with("b.rs")), "{drawn:?}");
}

#[test]
fn a_keystroke_shows_in_the_stream_before_the_rows_are_aligned_again() {
    use groove_types::{Edit, Motion};

    let mut app = opened();
    let mut ui = on_diff();
    ui.focus = crate::Focus::Workspace;
    let buffer = &mut app.workspace.opened.as_mut().expect("the open file").new;
    buffer.edit(&Edit::Move(Motion::To(Caret::new(0, 0))));
    buffer.edit(&Edit::Insert("typed".into()));
    let drawn = texts(&app, &ui);
    assert!(
        drawn.iter().any(|text| text.starts_with("typed")),
        "the rows read the buffer, not the last alignment: {drawn:?}"
    );
}
