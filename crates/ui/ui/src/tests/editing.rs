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
    ui.session.tab = Tab::Diff;
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
    let point = (hits.chars().left + 1.0, code.y + 1.0);
    let pressed = handle(
        Input::Press {
            x: point.0,
            y: point.1,
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
    assert!(ui.selecting, "the pointer is choosing");
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
    assert!(!ui.selecting, "and the release ends it");
}
