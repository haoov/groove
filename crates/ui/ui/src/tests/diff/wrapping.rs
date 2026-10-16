//! Long lines wrapped to the width, and the diff following the caret sideways.

use super::*;
use crate::input::{Key, Modifiers};
use groove_controllers::{Command, workspace};
use groove_types::{Caret, Edit, Motion};

/// The file view of `many(200)`, its lines wrapping.
fn wrapping() -> (AppState, Ui) {
    let mut app = many(200);
    let config = r#"{ "git": { "worktree_root": "~/code" }, "ui": { "wrap_code": true } }"#;
    app.config.config = Some(serde_json::from_str(config).expect("a config"));
    let mut ui = on_diff();
    (ui.session.tab, ui.focus) = (Tab::Files, crate::Focus::Workspace);
    (app, ui)
}

fn to(app: &mut AppState, caret: Caret) {
    let open = app.workspace.active_mut().expect("the open file");
    open.new.edit(&Edit::Move(Motion::To(caret)));
}

#[test]
fn a_long_line_wraps_into_rows_as_wide_as_the_view() {
    let (app, ui) = wrapping();
    let _ = view(&app, &ui, window(), &mut Fonts::embedded());
    let cols = ui.wrapping.cols();
    assert!(cols > 20, "the frame measured the width: {cols}");
    let (starts, _) = crate::views::session::diff::wrapping::wrap_of(&app, &ui).expect("on");
    let line = 199;
    assert_eq!(
        starts[line + 1] - starts[line],
        400usize.div_ceil(cols),
        "400 characters"
    );
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert_eq!(
        hits.extent(crate::hit::Scroller::Across),
        0.0,
        "nothing to scroll sideways"
    );
}

#[test]
fn down_steps_along_a_wrapped_line_before_the_next_one() {
    let (mut app, mut ui) = wrapping();
    let _ = view(&app, &ui, window(), &mut Fonts::embedded());
    let cols = ui.wrapping.cols();
    to(&mut app, Caret::new(199, 3));
    let moved = crate::tests::press(Key::Down, Modifiers::default(), &mut ui, &app);
    let wanted = Edit::Move(Motion::To(Caret::new(199, cols + 3)));
    assert_eq!(
        moved,
        [Command::Workspace(workspace::Command::Edit(wanted))]
    );
}

#[test]
fn a_click_on_a_wrapped_row_lands_along_its_line() {
    let (app, ui) = wrapping();
    let _ = view(&app, &ui, window(), &mut Fonts::embedded());
    let (starts, cols) = crate::views::session::diff::wrapping::wrap_of(&app, &ui).expect("on");
    let line = 30;
    assert!(
        starts[line + 1] - starts[line] > 1,
        "line 30 takes two rows"
    );
    let row = starts[line] + 1;
    let (path, at, from) =
        crate::views::session::diff::line_at(&app, &ui, (ui.session.face(), 0), row)
            .expect("a code row");
    assert_eq!((path.as_str(), at, from), ("src/lib.rs", line, cols));
}

#[test]
fn the_diff_follows_the_caret_sideways() {
    let mut app = with_files();
    let long = "y".repeat(300);
    shows(&mut app, "src/lib.rs", "x\n", &format!("x\n{long}\n"));
    let mut ui = on_diff();
    ui.focus = crate::Focus::Workspace;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    to(&mut app, Caret::new(1, 300));
    crate::input::follow(&mut ui, &app, &hits, window());
    assert!(ui.session.across() > 0.0, "the view went along the line");
}

#[test]
fn a_typed_note_wraps_at_the_note_s_width() {
    let pieces = crate::views::session::diff::wrap::pieces("one two three four", 9);
    assert_eq!(
        pieces,
        [(0, 0, 8), (0, 8, 14), (0, 14, 18)],
        "broken after a space"
    );
    let pieces = crate::views::session::diff::wrap::pieces("abcdefghij\nk", 4);
    assert_eq!(pieces, [(0, 0, 4), (0, 4, 8), (0, 8, 10), (1, 0, 1)]);
}

#[test]
fn wrapping_turns_on_from_settings_or_the_palette() {
    let (app, _) = wrapping();
    let labels: Vec<String> = crate::palette::entries(&app)
        .into_iter()
        .map(|one| one.label)
        .collect();
    assert!(
        labels.iter().any(|one| one == "Stop wrapping long lines"),
        "{labels:?}"
    );
    let ui = Ui::default();
    let rows = crate::views::settings::rows::rows(&app, &ui.settings);
    let wrap = rows
        .iter()
        .find(|one| one.label == "wrap long lines")
        .expect("the toggle");
    assert!(matches!(
        wrap.value,
        crate::views::settings::rows::Value::Toggle { on: true, .. }
    ));
}
