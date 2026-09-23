//! What the keyboard asks of the open buffer once the workspace holds it.

use groove_controllers::{Command, workspace};
use groove_types::{Edit, Motion};

use crate::input::{Key, Modifiers};
use crate::tests::diff::opened;
use crate::tests::press;
use crate::views::session::Tab;
use crate::{Focus, Ui};

const PLAIN: Modifiers = Modifiers {
    ctrl: false,
    shift: false,
    alt: false,
};

const CTRL: Modifiers = Modifiers {
    ctrl: true,
    shift: false,
    alt: false,
};

fn editing() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::File;
    ui.focus = Focus::Workspace;
    ui
}

/// What one key asks of the buffer.
fn asked(key: Key, mods: Modifiers) -> Vec<Command> {
    let app = opened();
    let mut ui = editing();
    press(key, mods, &mut ui, &app)
}

fn edit(key: Key, mods: Modifiers) -> Option<Edit> {
    match asked(key, mods).into_iter().next() {
        Some(Command::Workspace(workspace::Command::Edit(edit))) => Some(edit),
        _ => None,
    }
}

#[test]
fn a_character_is_put_in_where_the_caret_is() {
    assert_eq!(
        edit(Key::Char('x'), PLAIN),
        Some(Edit::Insert("x".into())),
        "and the buffer decides where that is"
    );
}

#[test]
fn the_keys_that_change_a_line_reach_the_buffer() {
    assert_eq!(edit(Key::Enter, PLAIN), Some(Edit::Newline));
    assert_eq!(edit(Key::Backspace, PLAIN), Some(Edit::Backspace));
    assert_eq!(edit(Key::Delete, PLAIN), Some(Edit::Delete));
}

#[test]
fn the_arrows_move_the_caret_in_the_document() {
    for (key, motion) in [
        (Key::Left, Motion::Left),
        (Key::Right, Motion::Right),
        (Key::Up, Motion::Up),
        (Key::Down, Motion::Down),
        (Key::Home, Motion::LineStart),
        (Key::End, Motion::LineEnd),
    ] {
        assert_eq!(edit(key, PLAIN), Some(Edit::Move(motion)), "{key:?}");
    }
}

#[test]
fn control_saves_and_undoes() {
    assert_eq!(
        asked(Key::Char('s'), CTRL),
        [Command::Workspace(workspace::Command::SaveFile)]
    );
    assert_eq!(edit(Key::Char('z'), CTRL), Some(Edit::Undo));
    assert_eq!(edit(Key::Char('y'), CTRL), Some(Edit::Redo));
}

#[test]
fn a_pane_that_is_not_the_workspace_never_edits() {
    let app = opened();
    for focus in [Focus::Rail, Focus::Sidebar] {
        let mut ui = editing();
        ui.focus = focus;
        let commands = press(Key::Char('x'), PLAIN, &mut ui, &app);
        assert!(
            !commands
                .iter()
                .any(|c| matches!(c, Command::Workspace(workspace::Command::Edit(_)))),
            "{focus:?} typed into the buffer"
        );
    }
}

#[test]
fn nothing_is_asked_of_a_buffer_that_is_not_open() {
    let app = crate::tests::full_app();
    let mut ui = editing();
    assert!(press(Key::Char('x'), PLAIN, &mut ui, &app).is_empty());
}

#[test]
fn shift_and_a_motion_holds_what_it_passes_over() {
    for (key, motion) in [
        (Key::Right, Motion::Right),
        (Key::Down, Motion::Down),
        (Key::End, Motion::LineEnd),
    ] {
        let mods = Modifiers {
            shift: true,
            ..PLAIN
        };
        assert_eq!(edit(key, mods), Some(Edit::Extend(motion)), "{key:?}");
    }
}

#[test]
fn control_a_holds_the_whole_file_and_tab_indents() {
    assert_eq!(edit(Key::Char('a'), CTRL), Some(Edit::SelectAll));
    assert_eq!(edit(Key::Tab, PLAIN), Some(Edit::Indent));
}

#[test]
fn a_drag_over_the_file_holds_more_of_it() {
    use crate::hit::Target;
    use crate::input::{Input, handle};
    use crate::tests::window;
    use crate::view;
    use groove_gfx::Fonts;

    let app = opened();
    let mut ui = editing();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let line = crate::tokens::Tokens::new(1.0).line;
    let point = (hits.chars().left + 1.0, code.y + line * 2.0 + 1.0);
    let pressed = handle(
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
    assert!(matches!(
        pressed.first(),
        Some(Command::Workspace(workspace::Command::Edit(Edit::Move(_))))
    ));
    assert!(
        ui.pointing(),
        "the window sends moves while the pointer leads something"
    );
    let dragged = handle(
        Input::Move {
            x: point.0 + hits.chars().advance * 3.0,
            y: point.1,
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert!(
        matches!(
            dragged.first(),
            Some(Command::Workspace(workspace::Command::Edit(Edit::Extend(
                _
            ))))
        ),
        "a move while down extends: {dragged:?}"
    );
    handle(Input::Release, &mut ui, &app, &hits, window());
    assert!(!ui.pointing(), "and the release ends it");
}

#[test]
fn control_carries_what_is_held() {
    for (key, wanted) in [
        (Key::Char('c'), workspace::Command::Copy),
        (Key::Char('x'), workspace::Command::Cut),
        (Key::Char('v'), workspace::Command::Paste),
    ] {
        assert_eq!(asked(key, CTRL), [Command::Workspace(wanted)], "{key:?}");
    }
}

#[test]
fn the_pointer_leads_nothing_until_it_goes_down_on_something() {
    let ui = Ui::default();
    assert!(!ui.pointing(), "a still pointer sends no moves");
    let app = opened();
    let mut ui = editing();
    let (_, hits) = crate::view(
        &app,
        &ui,
        crate::tests::window(),
        &mut groove_gfx::Fonts::embedded(),
    );
    let moved = crate::input::handle(
        crate::input::Input::Move { x: 200.0, y: 200.0 },
        &mut ui,
        &app,
        &hits,
        crate::tests::window(),
    );
    assert!(moved.is_empty(), "a move with nothing down asks nothing");
}

/// Presses at the same point, `apart` milliseconds between them.
fn clicks(times: usize, apart: u64) -> Vec<Command> {
    use crate::hit::Target;
    use crate::input::{Input, handle};
    use crate::tests::metrics;
    use groove_gfx::Fonts;

    let app = opened();
    let mut ui = editing();
    let (_, hits) = crate::view(&app, &ui, crate::tests::window(), &mut Fonts::embedded());
    let code = hits.rect_of(&Target::Code).expect("the rows are drawn");
    let line = crate::tokens::Tokens::new(1.0).line;
    let point = (hits.chars().left + 1.0, code.y + line * 2.0 + 1.0);
    let mut commands = Vec::new();
    for at in 0..times {
        let mut window = metrics(1280, 800, 1.0);
        window.tick = at as u64 * apart;
        commands = handle(
            Input::Press {
                x: point.0,
                y: point.1,
                mods: Default::default(),
            },
            &mut ui,
            &app,
            &hits,
            window,
        );
        handle(Input::Release, &mut ui, &app, &hits, window);
    }
    commands
}

/// The edits one press asked for.
fn edits(commands: Vec<Command>) -> Vec<Edit> {
    commands
        .into_iter()
        .filter_map(|command| match command {
            Command::Workspace(workspace::Command::Edit(edit)) => Some(edit),
            _ => None,
        })
        .collect()
}

#[test]
fn one_click_lands_the_caret_and_holds_nothing() {
    let asked = edits(clicks(1, 0));
    assert!(matches!(asked.as_slice(), [Edit::Move(_)]), "{asked:?}");
}

#[test]
fn two_clicks_hold_the_word_and_three_hold_the_line() {
    let two = edits(clicks(2, 50));
    assert!(
        matches!(two.as_slice(), [Edit::Move(_), Edit::SelectWord]),
        "{two:?}"
    );
    let three = edits(clicks(3, 50));
    assert!(
        matches!(three.as_slice(), [Edit::Move(_), Edit::SelectLine]),
        "{three:?}"
    );
}

#[test]
fn a_press_long_after_another_is_a_click_of_its_own() {
    let slow = edits(clicks(2, crate::tokens::CLICK_MS + 1));
    assert!(
        matches!(slow.as_slice(), [Edit::Move(_)]),
        "too late to carry the first one on: {slow:?}"
    );
}
