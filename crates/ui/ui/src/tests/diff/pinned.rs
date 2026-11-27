//! The lines held over the rows: the scopes above them.

use groove_controllers::AppState;

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
    ui.session.mode = crate::views::session::Mode::Files;
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
    ui.session.mode = crate::views::session::Mode::Files;
    assert_eq!(band(&app, &ui), "", "the scopes are in the rows themselves");
}

#[test]
fn a_scope_the_band_covers_is_pinned_at_once_and_not_a_scroll_later() {
    let app = nested();
    let mut ui = on_diff();
    ui.session.mode = crate::views::session::Mode::Files;
    let line = Tokens::new(1.0).code_line;
    *ui.session.scroll_mut() = 2.0 * line;
    let said = band(&app, &ui);
    assert!(
        said.contains("fn three()"),
        "the row under the band is named by it: {said}"
    );
}

#[test]
fn a_scope_stands_over_the_rows_as_soon_as_its_line_leaves_the_top() {
    let app = nested();
    let mut ui = on_diff();
    ui.session.mode = crate::views::session::Mode::Files;
    *ui.session.scroll_mut() = Tokens::new(1.0).code_line / 3.0;
    let said = band(&app, &ui);
    assert!(said.contains("mod one {"), "not a whole line later: {said}");
}
