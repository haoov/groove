//! The lines held over the rows: the file they belong to, and the scopes above them.

use groove_controllers::AppState;
use groove_types::DiffView;

use super::stream::both;
use super::{on_diff, view_of, with_files};
use crate::Ui;
use crate::base::hit::Target;
use crate::base::tokens::Tokens;

/// What the band standing over the rows says, its coloured runs joined.
fn band(app: &AppState, ui: &Ui) -> String {
    let (frame, _) = view_of(app, ui);
    let layers = frame.layers();
    match layers.len() > 1 {
        true => layers
            .iter()
            .skip(1)
            .flat_map(|layer| layer.texts.iter())
            .map(|run| run.text.as_str())
            .collect(),
        false => String::new(),
    }
}

/// One file deep enough to scroll inside a scope.
fn nested() -> AppState {
    let mut app = with_files();
    let body: String = (0..40)
        .map(|at| format!("            let value_{at} = {at};\n"))
        .collect();
    let before = format!(
        "mod one {{\n    impl Two {{\n        fn three() {{\n{body}        }}\n    }}\n}}\n"
    );
    let after = before.replace("let value_3 = 3;", "let value_3 = 33;");
    crate::tests::shows(&mut app, "src/lib.rs", &before, &after);
    app
}

#[test]
fn the_scopes_above_the_first_row_stand_over_it() {
    let app = nested();
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    ui.session.diff = Tokens::new(1.0).line * 20.0;
    let band = band(&app, &ui);
    for scope in ["mod one {", "impl Two {", "fn three() {"] {
        assert!(
            band.contains(scope.trim()),
            "{scope} stands over the rows: {band}"
        );
    }
}

#[test]
fn a_scope_already_on_screen_does_not_stand_over_it_as_well() {
    let app = nested();
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    assert_eq!(band(&app, &ui), "", "the scopes are in the rows themselves");
}

#[test]
fn the_stream_pins_the_file_it_stands_in() {
    let app = both();
    let mut ui = on_diff();
    let head = app.workspace.changes.head_of("src/b.rs").expect("the file");
    ui.session.diff = (head + 1) as f32 * Tokens::new(1.0).line;
    assert!(band(&app, &ui).contains("src/b.rs"), "{}", band(&app, &ui));
}

#[test]
fn the_pinned_head_stands_while_the_file_s_own_rows_pass_under_it() {
    let app = both();
    let mut ui = on_diff();
    let line = Tokens::new(1.0).line;
    let bands: Vec<f32> = (1..6)
        .map(|row| {
            ui.session.diff = row as f32 * line;
            let (_, hits) = view_of(&app, &ui);
            hits.rect_of(&Target::Pinned).map(|r| r.h).unwrap_or(0.0)
        })
        .collect();
    assert!(
        bands.iter().all(|high| *high > 0.0),
        "the file stays named while its rows pass: {bands:?}"
    );
}

#[test]
fn a_scope_the_band_covers_is_pinned_at_once_and_not_a_scroll_later() {
    let app = nested();
    let mut ui = on_diff();
    ui.session.view = DiffView::Editor;
    let line = Tokens::new(1.0).line;
    ui.session.diff = 2.0 * line;
    let said = band(&app, &ui);
    assert!(
        said.contains("fn three()"),
        "the row under the band is named by it: {said}"
    );
}

#[test]
fn the_diff_names_the_file_by_its_path_the_whole_way_down() {
    let mut app = with_files();
    crate::tests::changed_files(
        &mut app,
        &[
            ("src/a.rs", "one\ntwo\nthree\n", "ONE\ntwo\nthree\n"),
            ("src/b.rs", "one\ntwo\nthree\n", "ONE\ntwo\nthree\n"),
            ("lib/c.rs", "one\ntwo\nthree\n", "ONE\ntwo\nthree\n"),
        ],
    );
    let mut ui = on_diff();
    let line = Tokens::new(1.0).line;
    let said: Vec<String> = (0..16)
        .map(|row| {
            ui.session.diff = row as f32 * line;
            band(&app, &ui)
        })
        .collect();
    assert_eq!(said[0], "", "nothing has scrolled off yet");
    assert!(
        said[1..]
            .iter()
            .all(|one| one.starts_with("src") || one.starts_with("lib")),
        "no row after leaves the file out: {said:?}"
    );
    assert!(
        said.iter().any(|one| one.starts_with("lib/c.rs")),
        "the next file takes over: {said:?}"
    );
}

#[test]
fn a_file_that_comes_into_view_is_not_named_twice() {
    let mut app = with_files();
    crate::tests::changed_files(
        &mut app,
        &[
            (
                "src/a.rs",
                "one
two
three
",
                "ONE
two
three
",
            ),
            (
                "src/b.rs",
                "one
two
three
",
                "ONE
two
three
",
            ),
        ],
    );
    let mut ui = on_diff();
    let line = Tokens::new(1.0).line;
    let head = app.workspace.changes.head_of("src/b.rs").expect("the file");

    for row in [head - 1, head, head + 1] {
        ui.session.diff = row as f32 * line;
        let said = band(&app, &ui);
        let (_, hits) = view_of(&app, &ui);
        let under = hits
            .rect_of(&Target::Pinned)
            .map(|one| one.h / line)
            .unwrap_or(0.0);
        assert!(under <= 1.0, "one row at most stands over the rows");
        if said.starts_with("src/b.rs") {
            assert!(
                row >= head,
                "the file is named once its own head has passed"
            );
        }
    }
}
