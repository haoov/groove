//! The code surface scrolled sideways: the wheel, a click, and the caret.

use super::*;
use crate::hit::{Hits, Scroller};
use crate::input::Key;
use groove_controllers::{Command, workspace};
use groove_types::{Caret, Edit, Motion};

/// The file view of a file whose last line runs 400 characters.
fn wide() -> (AppState, Ui) {
    let app = many(200);
    let mut ui = on_diff();
    ui.session.tab = Tab::Files;
    ui.focus = crate::Focus::Workspace;
    (app, ui)
}

fn sideways(across: f32, ui: &mut Ui, app: &AppState, hits: &Hits) {
    let workspace = crate::layout::Layout::of(window(), ui).workspace;
    let delta = Delta::Pixels { across, down: 0.0 };
    let (x, y) = (workspace.x + 1.0, workspace.y + 1.0);
    handle(Input::Scroll { x, y, delta }, ui, app, hits, window());
}

#[test]
fn the_wheel_across_scrolls_the_text_up_to_the_widest_line() {
    let (app, mut ui) = wide();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    sideways(-80.0, &mut ui, &app, &hits);
    assert_eq!(ui.session.across(), 80.0);
    assert_eq!(ui.session.scroll(), 0.0, "it did not go down");

    sideways(-80_000.0, &mut ui, &app, &hits);
    let far = hits.extent(Scroller::Across);
    assert!(far > 80.0, "the widest line runs past the room: {far}");
    assert_eq!(ui.session.across(), far);
    sideways(80_000.0, &mut ui, &app, &hits);
    assert_eq!(ui.session.across(), 0.0);
}

#[test]
fn text_scrolled_across_moves_left_and_the_numbers_stay() {
    let (app, mut ui) = wide();
    let runs = |ui: &Ui| {
        let (frame, _) = view(&app, ui, window(), &mut Fonts::embedded());
        frame.layers()[0].texts.clone()
    };
    let at = |runs: &[groove_gfx::TextRun], text: &str| {
        runs.iter()
            .find(|run| run.text == text)
            .map(|run| run.x)
            .unwrap_or_else(|| panic!("{text} is drawn"))
    };
    let before = runs(&ui);
    ui.session.file_across = 40.0;
    let after = runs(&ui);
    assert_eq!(at(&after, "abab"), at(&before, "abab") - 40.0);
    assert_eq!(at(&after, "2"), at(&before, "2"), "the gutter stays");
}

#[test]
fn a_click_on_text_scrolled_across_lands_on_the_column_under_it() {
    let (app, mut ui) = wide();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let advance = hits.chars().advance;
    ui.session.file_across = 10.0 * advance;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let chars = hits.chars();
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let x = chars.left + ui.session.across() + advance * 2.0;
    let y = code.y + Tokens::new(1.0).line * 10.0 + 1.0;
    let mods = Default::default();
    let commands = handle(Input::Press { x, y, mods }, &mut ui, &app, &hits, window());
    let wanted = Edit::Move(Motion::To(Caret::new(10, 12)));
    assert_eq!(
        commands,
        [Command::Workspace(workspace::Command::Edit(wanted))]
    );
}

#[test]
fn a_caret_moved_off_the_view_brings_it_back() {
    let (mut app, mut ui) = wide();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let to = |app: &mut AppState, caret: Caret| {
        let open = app.workspace.active_mut().expect("the open file");
        open.new.edit(&Edit::Move(Motion::To(caret)));
    };
    to(&mut app, Caret::new(199, 400));
    crate::input::follow(&mut ui, &app, &hits, window());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let (line, advance) = (Tokens::new(1.0).line, hits.chars().advance);
    let row = 199.0 * line;
    assert_eq!(
        ui.session.scroll(),
        row + line - code.h,
        "the last row ends the view"
    );
    let room = code.right() - hits.chars().left - Tokens::new(1.0).md;
    assert_eq!(ui.session.across(), 401.0 * advance - room);

    to(&mut app, Caret::new(0, 0));
    crate::input::follow(&mut ui, &app, &hits, window());
    assert_eq!((ui.session.scroll(), ui.session.across()), (0.0, 0.0));
}

#[test]
fn a_caret_that_did_not_move_leaves_the_view_where_the_wheel_put_it() {
    let (app, mut ui) = wide();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    crate::input::follow(&mut ui, &app, &hits, window());
    ui.session.file = 400.0;
    ui.session.file_across = 80.0;
    crate::input::follow(&mut ui, &app, &hits, window());
    assert_eq!((ui.session.scroll(), ui.session.across()), (400.0, 80.0));
}

#[test]
fn a_found_match_far_along_its_line_opens_scrolled_to_it() {
    let (mut app, mut ui) = wide();
    let text = format!("{}o", " ".repeat(300));
    app.workspace.found = vec![groove_controllers::workspace_service::Found {
        path: "src/lib.rs".into(),
        line: 150,
        text,
        at: (300, 301),
    }];
    ui.session.file_across = 0.0;
    crate::tests::press(Key::Char('f'), crate::tests::CTRL_SHIFT, &mut ui, &app);
    crate::tests::press(Key::Char('o'), Default::default(), &mut ui, &app);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let row = hits
        .rect_of(&Target::Found(0))
        .expect("the match is listed");
    click(row, &mut ui, &app, &hits);
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let chars = hits.chars();
    let room = code.right() - chars.left - Tokens::new(1.0).md;
    assert_eq!(ui.session.file_across, 302.0 * chars.advance - room);
}
