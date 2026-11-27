//! The Diff's surface over the focused file, and its map.

use super::*;
use crate::tests::{changed_files, pressed, release};
use groove_controllers::{Command, workspace};
use groove_types::{Caret, LineMark};

const FILES: [(&str, &str, &str); 2] = [
    ("src/a.rs", "one\n", "ONE\n"),
    ("src/b.rs", "two\n", "TWO\n"),
];

pub(super) fn both() -> AppState {
    let mut app = with_files();
    changed_files(&mut app, &FILES);
    app
}

#[test]
fn a_click_in_a_file_that_is_not_open_opens_it_where_it_was_clicked() {
    let app = both();
    let mut ui = on_diff();
    let (_, hits) = view_of(&app, &ui);
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let tokens = Tokens::new(1.0);
    let point = (hits.chars().left + 1.0, code.y + tokens.code_line + 1.0);
    let commands = handle(
        Input::Press {
            x: point.0,
            y: point.1,
            mods: Default::default(),
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
            at: Some(groove_types::Selection::at(Caret::new(0, 0))),
        })],
        "the line that went, then the line that came"
    );
}

#[test]
fn a_keystroke_shows_in_the_stream_before_the_rows_are_aligned_again() {
    use groove_types::{Edit, Motion};

    let mut app = opened();
    let mut ui = on_diff();
    ui.focus = crate::Focus::Workspace;
    let buffer = &mut app.workspace.active_mut().expect("the open file").new;
    buffer.edit(&Edit::Move(Motion::To(Caret::new(0, 0))));
    buffer.edit(&Edit::Insert("typed".into()));
    let drawn = texts(&app, &ui);
    assert!(
        drawn.iter().any(|text| text.starts_with("typed")),
        "the rows read the buffer, not the last alignment: {drawn:?}"
    );
}

#[test]
fn the_map_holds_the_focused_file_s_band_and_a_lens_over_the_rows() {
    let app = both();
    let ui = on_diff();
    let (frame, hits) = view_of(&app, &ui);
    let column = hits.rect_of(&Target::Map).expect("the map is drawn");
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let inside = |quad: &groove_gfx::Quad| {
        quad.rect.x >= column.x && quad.rect.right() <= column.right() + 1.0
    };
    let quads: Vec<&groove_gfx::Quad> = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| inside(quad))
        .collect();
    let bands = quads.iter().filter(|q| q.color == styles.ground()).count();
    assert_eq!(bands, 1, "the focused file's band, over the column's own");
    let marks = quads
        .iter()
        .filter(|q| {
            q.color == styles.mark(LineMark::Added) || q.color == styles.mark(LineMark::Removed)
        })
        .count();
    assert_eq!(marks, 2, "one line out and one in");
    assert!(
        quads.iter().any(|q| q.color == styles.lens()),
        "the lens stands over it"
    );
}

#[test]
fn a_press_on_the_map_holds_the_rows_it_points_at() {
    let app = both();
    let mut ui = on_diff();
    let (_, hits) = view_of(&app, &ui);
    let column = hits.rect_of(&Target::Map).expect("the map is drawn");
    pressed(column.x + 1.0, column.bottom() - 1.0, &mut ui, &app, &hits);
    assert!(
        matches!(ui.held, Some(crate::Held::Lens)),
        "the lens follows the pointer"
    );
    assert_eq!(
        ui.session.diff,
        hits.extent(crate::hit::Scroller::Code),
        "the foot of the map is the end of the change"
    );
    release(&mut ui, &app, &hits);
    assert!(!matches!(ui.held, Some(crate::Held::Lens)));
}

#[test]
fn the_file_view_makes_the_map_the_file_s_own_scrollbar() {
    let app = many(200);
    let mut ui = on_diff();
    ui.session.mode = crate::views::session::Mode::Files;
    let (frame, hits) = view_of(&app, &ui);
    let column = hits.rect_of(&Target::Map).expect("the map is drawn");
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let per = column.h / 200.0;
    let quads: Vec<&groove_gfx::Quad> = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.rect.x >= column.x && quad.rect.right() <= column.right() + 1.0)
        .collect();

    let changed = quads
        .iter()
        .find(|quad| quad.color == styles.mark(LineMark::Changed))
        .expect("the line the change touched");
    assert!(
        (changed.rect.y - (column.y + 19.0 * per)).abs() <= per,
        "the mark sits where the line does"
    );

    let edges: Vec<f32> = quads
        .iter()
        .filter(|quad| quad.color == styles.lens())
        .map(|quad| quad.rect.y)
        .collect();
    let (top, foot) = (
        edges.iter().copied().fold(f32::MAX, f32::min),
        edges.iter().copied().fold(0.0, f32::max),
    );
    assert_eq!(top, column.y, "the file is at its start");
    assert!(foot - top < column.h, "the lens holds what the rows show");
}

#[test]
fn the_band_names_the_focused_file_in_the_diff_and_the_active_one_in_files() {
    let mut app = both();
    let mut ui = on_diff();
    let middle = crate::layout::Layout::of(window(), &ui).agent;
    let row = Tokens::new(1.0).row;
    let band = |app: &AppState, ui: &Ui| {
        let (frame, _) = view_of(app, ui);
        let top = middle.y + Tokens::new(1.0).sm;
        frame.layers()[0]
            .texts
            .iter()
            .filter(|run| run.x >= middle.x && run.x < middle.right())
            .filter(|run| run.y >= top && run.y < top + row)
            .map(|run| run.text.clone())
            .collect::<Vec<String>>()
    };
    let named = band(&app, &ui);
    assert!(named.iter().any(|text| text.ends_with("a.rs")), "{named:?}");

    crate::tests::shows(&mut app, "src/b.rs", FILES[1].1, FILES[1].2);
    changed_files(&mut app, &FILES);
    ui.session.mode = crate::views::session::Mode::Files;
    let named = band(&app, &ui);
    assert!(
        named.iter().any(|text| text.ends_with("b.rs")),
        "the Files mode names the active one: {named:?}"
    );
    assert!(
        !named.iter().any(|text| text.ends_with("a.rs")),
        "{named:?}"
    );
}

#[test]
fn no_rule_stands_between_the_gutters_and_the_code() {
    let app = both();
    let ui = on_diff();
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let tokens = Tokens::new(1.0);
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let surface = crate::layout::Layout::of(window(), &ui).agent;
    let gutters = surface.x..surface.right() - tokens.map;
    let upright = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.color == styles.line() && quad.rect.w == tokens.hairline)
        .any(|quad| gutters.contains(&quad.rect.x) && quad.rect.h >= tokens.code_line);
    assert!(
        !upright,
        "the numbers and the text stand apart by space alone"
    );
}
