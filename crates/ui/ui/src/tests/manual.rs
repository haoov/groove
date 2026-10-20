//! The manual section: folded to its bar, opened by `+`, and holding the keyboard once clicked.

use groove_controllers::{AppState, Command, shell};

use super::*;
use crate::Focus;
use crate::hit::Target;
use crate::layout::{Layout, grid_in};

fn session(app: &AppState) -> groove_types::SessionId {
    app.session.selected.clone().expect("a session is open")
}

#[test]
fn the_section_stands_folded_to_its_bar_until_asked() {
    let app = full_app();
    let ui = Ui::default();
    let hits = drawn_hits(&app, &ui);
    assert!(
        hits.rect_of(&Target::ShellNew).is_some(),
        "the bar offers a terminal"
    );
    let layout = Layout::of(window(), &ui);
    let bar = window().tokens().bar;
    assert_eq!(layout.manual.h, bar);
    assert_eq!(layout.workspace.bottom(), layout.manual.y);
}

#[test]
fn plus_opens_the_section_with_a_terminal_its_grid_holds() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = drawn_hits(&app, &ui);
    let plus = hits.rect_of(&Target::ShellNew).expect("the button");
    let asked = click(plus, &mut ui, &app, &hits);
    assert!(ui.session.manual, "the section opens");
    let tokens = window().tokens();
    let pane = Layout::of(window(), &ui).shell_panes(&tokens, 1)[0];
    let (cols, rows) = grid_in(pane, &tokens, window().cell);
    let open = shell::Command::Open {
        session: session(&app),
        cols,
        rows,
    };
    assert_eq!(asked, [Command::Shell(open)]);
}

#[test]
fn a_clicked_terminal_takes_the_keys_and_the_paste() {
    let mut app = full_app();
    let id = app.shell.reserve(&session(&app), false);
    let mut ui = Ui::default();
    ui.session.manual = true;
    let hits = drawn_hits(&app, &ui);
    let grid = hits.rect_of(&Target::Shell(id)).expect("its grid");
    click(grid, &mut ui, &app, &hits);
    assert_eq!(ui.focus, Focus::Terminal);
    let typed = press(Key::Char('a'), Modifiers::default(), &mut ui, &app);
    let send = shell::Command::Send {
        session: session(&app),
        id,
        bytes: b"a".to_vec(),
    };
    assert_eq!(typed, [Command::Shell(send)]);
    let pasted = handle(Input::Paste("ls".into()), &mut ui, &app, &hits, window());
    let paste = shell::Command::Paste {
        session: session(&app),
        id,
        text: "ls".into(),
    };
    assert_eq!(pasted, [Command::Shell(paste)]);
}

#[test]
fn a_split_tab_stands_its_panes_side_by_side_and_a_click_asks_for_that_one() {
    let mut app = full_app();
    let left = app.shell.reserve(&session(&app), false);
    let right = app.shell.reserve(&session(&app), true);
    let mut ui = Ui::default();
    ui.session.manual = true;
    let hits = drawn_hits(&app, &ui);
    let (one, two) = (
        hits.rect_of(&Target::Shell(left)).expect("the left pane"),
        hits.rect_of(&Target::Shell(right)).expect("the right pane"),
    );
    assert!(one.right() <= two.x, "left stands left of right");
    let asked = click(one, &mut ui, &app, &hits);
    let focus = shell::Command::Focus {
        session: session(&app),
        id: left,
    };
    assert_eq!(asked.first(), Some(&Command::Shell(focus)));
    app.shell.focus(&session(&app), left);
    let hits = drawn_hits(&app, &ui);
    assert_eq!(
        hits.rect_of(&Target::Shell(left)),
        Some(one),
        "and it stays where it was"
    );
}

#[test]
fn an_open_section_takes_its_height_from_the_foot_of_the_workspace() {
    let mut ui = Ui::default();
    ui.session.manual = true;
    let layout = Layout::of(window(), &ui);
    let tall = (ui.split.manual * window().tokens().scale).floor();
    assert_eq!(layout.manual.h, tall);
    assert_eq!(layout.manual.bottom(), layout.window.bottom());
    assert_eq!(layout.workspace.bottom(), layout.manual.y);
}

#[test]
fn a_press_on_a_terminal_selects_and_letting_go_copies() {
    let mut app = full_app();
    let id = app.shell.reserve(&session(&app), false);
    let mut ui = Ui::default();
    ui.session.manual = true;
    let hits = drawn_hits(&app, &ui);
    let grid = hits.rect_of(&Target::Shell(id)).expect("its grid");
    let asked = click(grid, &mut ui, &app, &hits);
    let said: Vec<&str> = asked.iter().map(|one| one.id()).collect();
    assert_eq!(said, ["shell.focus", "shell.select"]);
    assert_eq!(
        ui.held,
        Some(crate::Held::Selecting(crate::Grid::Shell(id)))
    );
    let (x, y) = (grid.x + grid.w / 2.0 + 40.0, grid.y + grid.h / 2.0);
    let carried = drag_at(x, y, &mut ui, &app, &hits);
    let said: Vec<&str> = carried.iter().map(|one| one.id()).collect();
    assert_eq!(
        said,
        ["shell.select"],
        "it carries while the button is down"
    );
    let let_go = handle(Input::Release, &mut ui, &app, &hits, window());
    let copy = shell::Command::Copy {
        session: session(&app),
        id,
    };
    assert_eq!(let_go, [Command::Shell(copy)]);
}

#[test]
fn the_copy_chord_on_a_terminal_copies_that_terminal() {
    let mut app = full_app();
    let id = app.shell.reserve(&session(&app), false);
    app.shell.focus(&session(&app), id);
    let mut ui = Ui {
        focus: Focus::Terminal,
        ..Ui::default()
    };
    let copied = press(Key::Char('c'), CTRL_SHIFT, &mut ui, &app);
    let copy = shell::Command::Copy {
        session: session(&app),
        id,
    };
    assert_eq!(copied, [Command::Shell(copy)]);
}

#[test]
fn a_program_that_reads_the_mouse_hears_it_and_shift_selects_instead() {
    let mut app = full_app();
    let id = app.shell.reserve(&session(&app), false);
    let reader = super::agent::mouse_reader();
    app.shell.started(&session(&app), id, Ok(reader));
    let mut ui = Ui::default();
    ui.session.manual = true;
    let hits = drawn_hits(&app, &ui);
    let grid = hits.rect_of(&Target::Shell(id)).expect("its grid");
    let (x, y) = (grid.x + grid.w / 2.0, grid.y + grid.h / 2.0);
    let act = |input: Input, ui: &mut Ui| -> Vec<&'static str> {
        let out = handle(input, ui, &app, &hits, window());
        out.iter().map(|one| one.id()).collect()
    };
    let press = |shift| Input::Press {
        x,
        y,
        mods: Modifiers {
            shift,
            ..Modifiers::default()
        },
    };
    assert_eq!(act(press(false), &mut ui), ["shell.focus", "shell.click"]);
    let moved = Input::Move { x: x + 40.0, y };
    assert_eq!(act(moved.clone(), &mut ui), ["shell.drag"]);
    assert_eq!(act(Input::Release, &mut ui), ["shell.click"]);
    assert_eq!(act(press(true), &mut ui), ["shell.focus", "shell.select"]);
    assert_eq!(
        act(moved, &mut ui),
        ["shell.select"],
        "shift drags a selection"
    );
    assert_eq!(act(Input::Release, &mut ui), ["shell.copy"]);
}
