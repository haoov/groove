//! The commit message answers the keys a note does: its selection, its undo, the clipboard.

use groove_controllers::{AppState, Command, cluster, workspace};
use groove_types::Edit;

use super::{file, listed, sidebar};
use crate::Ui;
use crate::hit::Target;
use crate::input::{Key, Modifiers};
use crate::tests::window;

fn ctrl() -> Modifiers {
    Modifiers {
        ctrl: true,
        ..Modifiers::default()
    }
}

fn shift() -> Modifiers {
    Modifiers {
        shift: true,
        ..Modifiers::default()
    }
}

/// The box holding the keyboard over a message that says `text`.
fn composing(text: &str) -> (AppState, Ui) {
    let mut app = listed(&[file("a.txt", true)]);
    app.workspace.message.edit(&Edit::Insert(text.into()));
    let mut ui = sidebar();
    ui.focus = crate::Focus::Sidebar;
    ui.session.composing = true;
    (app, ui)
}

/// One keystroke, its edits to the message done as the controller does them.
fn key(key: Key, mods: Modifiers, ui: &mut Ui, app: &mut AppState) -> Vec<Command> {
    let commands = crate::tests::press(key, mods, ui, app);
    for command in &commands {
        if let Command::Workspace(workspace::Command::Message(edit)) = command {
            app.workspace.message.edit(edit);
        }
    }
    commands
}

fn copied(text: &str) -> Command {
    Command::Cluster(cluster::Command::CopyValue { text: text.into() })
}

#[test]
fn shift_and_an_arrow_hold_the_words_and_a_cut_takes_them_to_the_clipboard() {
    let (mut app, mut ui) = composing("fix(a): leaks");
    for _ in 0..5 {
        key(Key::Left, shift(), &mut ui, &mut app);
    }
    assert_eq!(app.workspace.message.selected(), "leaks");
    let cut = key(Key::Char('x'), ctrl(), &mut ui, &mut app);
    let deleted = Command::Workspace(workspace::Command::Message(Edit::Delete));
    assert_eq!(cut, [copied("leaks"), deleted]);
    assert_eq!(app.workspace.message.text(), "fix(a): ");
}

#[test]
fn an_undo_brings_the_words_back_and_a_redo_takes_them_again() {
    let (mut app, mut ui) = composing("fix(a): it");
    key(Key::Backspace, Modifiers::default(), &mut ui, &mut app);
    key(Key::Char('z'), ctrl(), &mut ui, &mut app);
    assert_eq!(app.workspace.message.text(), "fix(a): it");
    key(Key::Char('y'), ctrl(), &mut ui, &mut app);
    assert_eq!(app.workspace.message.text(), "fix(a): i");
    key(Key::Char('z'), ctrl(), &mut ui, &mut app);
    let both = Modifiers {
        shift: true,
        ..ctrl()
    };
    key(Key::Char('z'), both, &mut ui, &mut app);
    assert_eq!(
        app.workspace.message.text(),
        "fix(a): i",
        "ctrl+shift+z redoes"
    );
}

#[test]
fn select_all_then_copy_leaves_the_message_as_it_was() {
    let (mut app, mut ui) = composing("fix(a): it\nthe body");
    key(Key::Char('a'), ctrl(), &mut ui, &mut app);
    let copy = key(Key::Char('c'), ctrl(), &mut ui, &mut app);
    assert_eq!(copy, [copied("fix(a): it\nthe body")]);
    assert_eq!(app.workspace.message.text(), "fix(a): it\nthe body");
    let nothing = key(Key::Right, Modifiers::default(), &mut ui, &mut app);
    assert_eq!(nothing.len(), 1, "a plain arrow lets the words go");
    assert!(key(Key::Char('c'), ctrl(), &mut ui, &mut app).is_empty());
}

#[test]
fn shift_and_enter_is_a_line_too() {
    let (mut app, mut ui) = composing("fix(a): it");
    key(Key::Enter, shift(), &mut ui, &mut app);
    assert_eq!(app.workspace.message.text(), "fix(a): it\n");
}

#[test]
fn the_box_shades_what_is_held_the_way_a_note_does() {
    let (mut app, mut ui) = composing("fix(a): it");
    let styles = groove_ui_kit::base::style::Styles::new(
        app.config.theme(),
        groove_ui_kit::base::tokens::Tokens::new(1.0),
    );
    let count = |app: &AppState, ui: &Ui| {
        let (frame, hits) = crate::view(app, ui, window(), &mut groove_gfx::Fonts::embedded());
        let box_ = hits.rect_of(&Target::Message).expect("the box");
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.held() && box_.contains(quad.rect.x, quad.rect.y))
            .count()
    };
    let before = count(&app, &ui);
    key(Key::Home, shift(), &mut ui, &mut app);
    assert_eq!(count(&app, &ui), before + 1, "the words held are shaded");
}

#[test]
fn alt_and_an_arrow_move_the_caret_with_or_without_shift() {
    let (mut app, mut ui) = composing("fix(a): it");
    let alt = Modifiers {
        alt: true,
        ..Modifiers::default()
    };
    let moved = key(Key::Left, alt, &mut ui, &mut app);
    let left = Edit::Move(groove_types::Motion::Left);
    assert_eq!(
        moved,
        [Command::Workspace(workspace::Command::Message(left))]
    );
    let held = Modifiers { shift: true, ..alt };
    key(Key::Home, held, &mut ui, &mut app);
    assert_eq!(
        app.workspace.message.caret(),
        groove_types::Caret::new(0, 0)
    );
    assert_eq!(app.workspace.message.selected(), "", "alt does not hold");
}

#[test]
fn alt_and_backspace_or_delete_take_a_character() {
    let (mut app, mut ui) = composing("fix(a): it");
    let alt = Modifiers {
        alt: true,
        ..Modifiers::default()
    };
    key(Key::Backspace, alt, &mut ui, &mut app);
    assert_eq!(app.workspace.message.text(), "fix(a): i");
    key(Key::Home, Modifiers::default(), &mut ui, &mut app);
    key(Key::Delete, alt, &mut ui, &mut app);
    assert_eq!(app.workspace.message.text(), "ix(a): i");
}

#[test]
fn ctrl_and_an_arrow_or_a_deletion_leave_the_message_as_it_was() {
    let (mut app, mut ui) = composing("fix(a): it");
    let keys = [Key::Left, Key::Home, Key::Backspace, Key::Delete, Key::End];
    for one in keys {
        key(one, ctrl(), &mut ui, &mut app);
    }
    assert_eq!(app.workspace.message.text(), "fix(a): it");
    assert_eq!(
        app.workspace.message.caret(),
        groove_types::Caret::new(0, 10)
    );
}
