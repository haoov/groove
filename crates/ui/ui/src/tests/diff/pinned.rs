//! The lines held over the rows: the file they belong to, and the scopes above them.

use groove_controllers::AppState;
use groove_controllers::workspace_service::HEAD_ROWS;

use super::stream::both;
use super::{on_diff, view_of, with_files};
use crate::Ui;
use crate::hit::Target;
use groove_ui_kit::base::tokens::Tokens;

/// What the band standing over the rows says, its coloured runs joined.
fn band(app: &AppState, ui: &Ui) -> String {
    let (frame, hits) = view_of(app, ui);
    let Some(pinned) = hits.rect_of(&Target::Pinned) else {
        return String::new();
    };
    frame
        .layers()
        .iter()
        .skip(1)
        .flat_map(|layer| layer.texts.iter())
        .filter(|run| pinned.contains(run.x, run.y))
        .map(|run| run.text.as_str())
        .collect()
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
    ui.session.tab = crate::views::session::Tab::Files;
    *ui.session.scroll_mut() = Tokens::new(1.0).code_line * 20.0;
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
    ui.session.tab = crate::views::session::Tab::Files;
    assert_eq!(band(&app, &ui), "", "the scopes are in the rows themselves");
}

#[test]
fn the_stream_pins_the_file_it_stands_in() {
    let app = both();
    let mut ui = on_diff();
    let head = app.workspace.changes.head_of("src/b.rs").expect("the file");
    *ui.session.scroll_mut() = (head + HEAD_ROWS) as f32 * Tokens::new(1.0).code_line;
    let said = band(&app, &ui);
    assert!(said.contains("b.rs") && said.contains("src"), "{said}");
}

#[test]
fn the_pinned_head_stands_while_the_file_s_own_rows_pass_under_it() {
    let app = both();
    let mut ui = on_diff();
    let line = Tokens::new(1.0).code_line;
    let bands: Vec<f32> = (1..6)
        .map(|row| {
            *ui.session.scroll_mut() = row as f32 * line;
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
    ui.session.tab = crate::views::session::Tab::Files;
    let line = Tokens::new(1.0).code_line;
    *ui.session.scroll_mut() = 2.0 * line;
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
    let line = Tokens::new(1.0).code_line;
    let (said, held): (Vec<String>, Vec<bool>) = (0..16)
        .map(|row| {
            *ui.session.scroll_mut() = row as f32 * line;
            let (_, hits) = view_of(&app, &ui);
            (band(&app, &ui), hits.rect_of(&Target::Pinned).is_some())
        })
        .unzip();
    assert_eq!(said[0], "", "nothing has scrolled off yet");
    assert!(
        held[1..].iter().all(|one| *one),
        "no row after leaves the file out: {said:?}"
    );
    assert!(
        said.iter()
            .any(|one| one.contains("c.rs") && one.contains("lib")),
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
    let line = Tokens::new(1.0).code_line;
    let head = app.workspace.changes.head_of("src/b.rs").expect("the file");

    for row in [head - 1, head, head + 1] {
        *ui.session.scroll_mut() = row as f32 * line;
        let said = band(&app, &ui);
        let (_, hits) = view_of(&app, &ui);
        let under = hits
            .rect_of(&Target::Pinned)
            .map(|one| one.h / line)
            .unwrap_or(0.0);
        assert!(
            under <= HEAD_ROWS as f32,
            "one head at most stands over the rows"
        );
        if said.contains("b.rs") {
            assert!(
                row >= head,
                "the file is named once its own head has passed"
            );
        }
    }
}

#[test]
fn a_scope_stands_over_the_rows_as_soon_as_its_line_leaves_the_top() {
    let app = nested();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    *ui.session.scroll_mut() = Tokens::new(1.0).code_line / 3.0;
    let said = band(&app, &ui);
    assert!(said.contains("mod one {"), "not a whole line later: {said}");
}

#[test]
fn the_pinned_head_stands_two_rows_tall_over_a_shadow() {
    let app = both();
    let mut ui = on_diff();
    let line = Tokens::new(1.0).code_line;
    *ui.session.scroll_mut() = line;
    let (frame, hits) = view_of(&app, &ui);
    let pinned = hits.rect_of(&Target::Pinned).expect("a pinned head");
    assert_eq!(pinned.h, line * HEAD_ROWS as f32);
    let shaded = frame
        .layers()
        .iter()
        .flat_map(|layer| layer.quads.iter())
        .filter(|quad| quad.shape.blur > 0.0)
        .any(|quad| quad.rect.y < pinned.bottom() && quad.rect.bottom() > pinned.bottom());
    assert!(shaded, "a shadow falls from its bottom edge");
}

#[test]
fn the_next_head_pushes_the_pinned_one_out() {
    let app = both();
    let mut ui = on_diff();
    let line = Tokens::new(1.0).code_line;
    let next = app.workspace.changes.head_of("src/b.rs").expect("the file");
    *ui.session.scroll_mut() = (next - 1) as f32 * line;
    let (_, hits) = view_of(&app, &ui);
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let pinned = hits.rect_of(&Target::Pinned).expect("a pinned head");
    assert_eq!(pinned.y, code.y, "what is left of it stands at the top");
    assert_eq!(pinned.h, line, "half of it is pushed out");
}
