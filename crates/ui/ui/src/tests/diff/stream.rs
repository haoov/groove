//! The diff tab as one surface over every changed file.

use super::*;
use crate::tests::{changed_files, pressed, release};
use groove_controllers::workspace_service::HEAD_ROWS;
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
fn every_changed_file_draws_under_a_row_naming_it() {
    let ui = on_diff();
    let (frame, hits) = view_of(&both(), &ui);
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let runs: Vec<_> = frame.layers()[0]
        .texts
        .iter()
        .filter(|run| code.contains(run.x, run.y))
        .collect();
    let drawn: Vec<String> = runs.iter().map(|run| run.text.clone()).collect();
    for (path, before, after) in FILES {
        let (folder, name) = path.rsplit_once('/').expect("a folder");
        let named = runs.iter().find(|run| run.text == name).expect("its name");
        assert!(
            runs.iter()
                .any(|run| run.text == folder && run.y == named.y),
            "its folder stands beside its name, never on a row of its own: {drawn:?}"
        );
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
    let point = (
        hits.chars().left + 1.0,
        code.y
            + tokens.code_line * (groove_controllers::workspace_service::HEAD_ROWS + 1) as f32
            + 1.0,
    );
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
        "the head, the line that went, then the line that came"
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
    assert_eq!(ui.session.diff, head as f32 * Tokens::new(1.0).code_line);
}

#[test]
fn the_header_names_the_file_the_surface_stands_in() {
    let app = both();
    let mut ui = on_diff();
    let head = app.workspace.changes.head_of("src/b.rs").expect("the file");
    ui.session.diff = head as f32 * Tokens::new(1.0).code_line;
    let drawn = texts(&app, &ui);
    assert!(drawn.iter().any(|text| text.ends_with("b.rs")), "{drawn:?}");
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
fn the_map_holds_a_band_for_every_file_and_a_lens_over_the_rows() {
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
    ui.session.tab = crate::views::session::Tab::Files;
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
fn a_click_on_a_file_head_folds_it() {
    let app = both();
    let mut ui = on_diff();
    let (_, hits) = view_of(&app, &ui);
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let head = (hits.chars().left + 1.0, code.y + 1.0);
    let commands = handle(
        Input::Press {
            x: head.0,
            y: head.1,
            mods: Default::default(),
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
        "the first row is the first file's head"
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
    let line = Tokens::new(1.0).code_line;
    ui.session.diff = line * 20.0;
    let (_, hits) = view_of(&app, &ui);
    let band = hits.rect_of(&Target::Pinned).expect("the band stands");
    let commands = handle(
        Input::Press {
            x: band.x + band.w / 2.0,
            y: band.bottom() - 1.0,
            mods: Default::default(),
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
        .selected_worktree()
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
fn a_file_read_dims_its_head() {
    let mut app = both();
    let worktree = selected(&app);
    let id = app.session.selected.clone().expect("a session");
    app.session
        .get_mut(&id)
        .expect("the row")
        .mark(&worktree, "src/a.rs", true);
    let ui = on_diff();
    let (frame, hits) = view_of(&app, &ui);
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let column = hits.rect_of(&Target::Map).expect("the map");
    let dimmed = frame.layers()[0]
        .quads
        .iter()
        .filter(|quad| quad.rect.x == column.x && quad.rect.w == column.w)
        .filter(|quad| quad.color == styles.hover())
        .count();
    assert_eq!(dimmed, 1, "the band of the file that was read");
    let code = hits.rect_of(&Target::Code).expect("the rows");
    let faint = frame.layers()[0]
        .texts
        .iter()
        .filter(|run| code.contains(run.x, run.y))
        .find(|run| run.text == "a.rs")
        .expect("its head");
    assert_eq!(
        faint.style.color,
        styles.color(groove_ui_kit::base::style::Role::Faint),
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
        [Command::Workspace(workspace::Command::Fold {
            path: "src/b.rs".into()
        })],
        "its rows come back, and no tab opens for it"
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
    let line = Tokens::new(1.0).code_line;
    assert_eq!(head.h, line * HEAD_ROWS as f32, "a head takes two rows");
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
fn the_header_names_the_active_file_in_files_and_none_in_the_diff() {
    let mut app = both();
    let mut ui = on_diff();
    let workspace = crate::layout::Layout::of(window(), &ui).workspace;
    let row = Tokens::new(1.0).row;
    let header = |app: &AppState, ui: &Ui| {
        let strip = match ui.session.tab {
            crate::views::session::Tab::Files => row + Tokens::new(1.0).sm,
            _ => 0.0,
        };
        let (frame, _) = view_of(app, ui);
        frame.layers()[0]
            .texts
            .iter()
            .filter(|run| run.x >= workspace.x && run.x < workspace.right())
            .filter(|run| {
                let top = workspace.y + strip;
                run.y >= top && run.y < top + row
            })
            .map(|run| run.text.clone())
            .collect::<Vec<String>>()
    };
    assert!(
        !header(&app, &ui).iter().any(|text| text.ends_with(".rs")),
        "the diff's header names no file, its heads do: {:?}",
        header(&app, &ui)
    );

    crate::tests::shows(&mut app, "src/b.rs", FILES[1].1, FILES[1].2);
    changed_files(&mut app, &FILES);
    ui.session.tab = crate::views::session::Tab::Files;
    let named = header(&app, &ui);
    assert!(
        named.iter().any(|text| text.ends_with("b.rs")),
        "the Files tab names the active one: {named:?}"
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
    let surface = crate::layout::Layout::of(window(), &ui).workspace;
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
