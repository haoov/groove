//! The diff tab as one surface over every changed file.

use super::*;
use crate::tests::{changed_files, pressed, release};
use groove_controllers::{Command, workspace};
use groove_types::{Caret, LineMark};

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
    assert_eq!(
        drawn.iter().filter(|text| *text == "src").count(),
        1,
        "the directory is named once: {drawn:?}"
    );
    for (path, before, after) in FILES {
        let name = path.rsplit('/').next().expect("a name");
        assert!(drawn.iter().any(|text| text == name), "{name}: {drawn:?}");
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
    let point = (hits.chars().left + 1.0, code.y + tokens.line * 3.0 + 1.0);
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
            at: Some(groove_types::Selection::at(Caret::new(0, 0))),
        })],
        "the band, the head, the line that went, then the line that came"
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
    ui.session.view = DiffView::File;
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
    ui.session.view = DiffView::File;
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
fn the_map_holds_a_band_for_every_file_and_a_lens_over_the_rows() {
    let app = both();
    let ui = on_diff();
    let (frame, hits) = view_of(&app, &ui);
    let column = hits.rect_of(&Target::Map).expect("the map is drawn");
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let inside = |quad: &groove_gfx::Quad| {
        quad.rect.x >= column.x && quad.rect.right() <= column.right() + 1.0
    };
    let quads: Vec<&groove_gfx::Quad> = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| inside(quad))
        .collect();
    let bands = quads.iter().filter(|q| q.color == styles.ground()).count();
    assert_eq!(bands, 2, "one band a file, over the column's own");
    let marks = quads
        .iter()
        .filter(|q| {
            q.color == styles.mark(LineMark::Added) || q.color == styles.mark(LineMark::Removed)
        })
        .count();
    assert_eq!(marks, 4, "one line out and one in, twice over");
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
    assert!(ui.mapping, "the lens follows the pointer");
    assert_eq!(
        ui.session.diff,
        hits.extent(crate::hit::Scroller::Code),
        "the foot of the map is the end of the change"
    );
    release(&mut ui, &app, &hits);
    assert!(!ui.mapping);
}

#[test]
fn the_file_view_makes_the_map_the_file_s_own_scrollbar() {
    let app = many(200);
    let mut ui = on_diff();
    ui.session.view = DiffView::File;
    let (frame, hits) = view_of(&app, &ui);
    let column = hits.rect_of(&Target::Map).expect("the map is drawn");
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
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
fn a_click_on_a_file_head_folds_it() {
    let app = both();
    let mut ui = on_diff();
    let (_, hits) = view_of(&app, &ui);
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let tokens = Tokens::new(1.0);
    let head = (hits.chars().left + 1.0, code.y + tokens.line + 1.0);
    let commands = handle(
        Input::Press {
            x: head.0,
            y: head.1,
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Fold {
            path: "src/a.rs".into()
        })],
        "the row under the band is the first file's head"
    );
}

/// One file whose every line changed, so its rows run past the window.
fn busy() -> AppState {
    let mut app = with_files();
    let before: String = (0..60)
        .map(|at| format!("let value_{at} = {at};\n"))
        .collect();
    let after = before.replace(" = ", " = 1 + ");
    crate::tests::shows(&mut app, "src/lib.rs", &before, &after);
    app
}

#[test]
fn folding_from_the_pinned_head_lands_on_the_file_it_shut() {
    let app = busy();
    let mut ui = on_diff();
    let line = Tokens::new(1.0).line;
    ui.session.diff = line * 20.0;
    let (_, hits) = view_of(&app, &ui);
    let band = hits.rect_of(&Target::Pinned).expect("the band stands");
    let commands = handle(
        Input::Press {
            x: band.x + band.w / 2.0,
            y: band.y + 1.0,
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Fold {
            path: "src/lib.rs".into()
        })]
    );
    let head = app
        .workspace
        .changes
        .head_of("src/lib.rs")
        .expect("its head");
    assert_eq!(
        ui.session.diff,
        head as f32 * line,
        "the rows it shut are gone, so it lands on the head itself"
    );
}

/// The worktree the fixture's session has selected.
fn selected(app: &AppState) -> groove_types::WorktreeId {
    app.session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|worktree| worktree.id.clone())
        .expect("the fixture has a worktree")
}

#[test]
fn the_mark_on_a_file_head_says_it_is_read() {
    let app = both();
    let mut ui = on_diff();
    let (_, hits) = view_of(&app, &ui);
    let box_ = hits
        .rect_of(&Target::Read("src/a.rs".into()))
        .expect("the head carries a mark");
    let commands = click(box_, &mut ui, &app, &hits);
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::MarkRead {
            path: "src/a.rs".into()
        })]
    );
}

#[test]
fn a_file_read_dims_its_head_and_its_band() {
    let mut app = both();
    let worktree = selected(&app);
    let id = app.session.selected.clone().expect("a session");
    app.session
        .get_mut(&id)
        .expect("the row")
        .mark(&worktree, "src/a.rs", true);
    let ui = on_diff();
    let (frame, hits) = view_of(&app, &ui);
    let styles = crate::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let column = hits.rect_of(&Target::Map).expect("the map");
    let dimmed = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.rect.x == column.x && quad.rect.w == column.w)
        .filter(|quad| quad.color == styles.hover())
        .count();
    assert_eq!(dimmed, 1, "the band of the file that was read");
    let faint = frame.layers()[0]
        .texts
        .iter()
        .find(|run| run.text == "a.rs")
        .expect("its head");
    assert_eq!(
        faint.style.color,
        styles.color(crate::style::Role::Faint),
        "its name reads quiet"
    );
}

#[test]
fn a_file_picked_in_the_sidebar_is_shown_again_when_it_was_folded() {
    let mut app = both();
    let mut ui = on_diff();
    app.workspace.changes.fold("src/b.rs");
    let (_, hits) = view_of(&app, &ui);
    let row = hits
        .rect_of(&Target::File("src/b.rs".into()))
        .expect("the sidebar lists it");
    let commands = click(row, &mut ui, &app, &hits);
    assert_eq!(
        commands,
        [
            Command::Workspace(workspace::Command::Fold {
                path: "src/b.rs".into()
            }),
            Command::Workspace(workspace::Command::OpenFile {
                path: "src/b.rs".into(),
                at: None
            }),
        ],
        "it opens, and its rows come back"
    );
}

#[test]
fn a_file_head_says_it_can_be_clicked() {
    let app = both();
    let ui = on_diff();
    let (_, hits) = view_of(&app, &ui);
    let head = hits
        .rect_of(&Target::Head("src/a.rs".into()))
        .expect("the head takes the pointer");
    let inside = (head.x + head.w / 2.0, head.y + head.h / 2.0);
    assert_eq!(
        hits.cursor_at(inside.0, inside.1),
        crate::hit::Cursor::Pointer,
        "not the text cursor the rows carry"
    );
    let rows = hits.rect_of(&Target::Code).expect("the rows");
    let under = (inside.0, head.bottom() + 1.0);
    assert!(rows.contains(under.0, under.1));
    assert_eq!(hits.cursor_at(under.0, under.1), crate::hit::Cursor::Text);
}

#[test]
fn the_header_names_the_file_that_is_open() {
    let mut app = both();
    let ui = on_diff();
    let workspace = crate::layout::Layout::of(window(), &ui).workspace;
    let row = Tokens::new(1.0).row;
    let header = |app: &AppState| {
        let (frame, _) = view_of(app, &ui);
        frame.layers()[0]
            .texts
            .iter()
            .filter(|run| run.x >= workspace.x && run.x < workspace.right())
            .filter(|run| run.y >= workspace.y + row && run.y < workspace.y + row * 2.0)
            .map(|run| run.text.clone())
            .collect::<Vec<String>>()
    };
    assert!(
        header(&app).iter().any(|text| text.ends_with("a.rs")),
        "with nothing open it names the file the rows start on: {:?}",
        header(&app)
    );

    crate::tests::shows(&mut app, "src/b.rs", FILES[1].1, FILES[1].2);
    changed_files(&mut app, &FILES);
    let named = header(&app);
    assert!(
        named.iter().any(|text| text.ends_with("b.rs")),
        "and once a file is open, that one: {named:?}"
    );
    assert!(
        !named.iter().any(|text| text.ends_with("a.rs")),
        "{named:?}"
    );
}
