//! The note being typed is a buffer: its caret, what it holds, its undo, the clipboard.

use super::*;
use crate::tests::typed;
use groove_controllers::{Command, workspace};

fn chord(key: Key, mods: Modifiers, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let input = Input::Key { key, mods };
    handle(input, ui, app, &crate::hit::Hits::default(), window())
}

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

/// A note open on line 1 of the file view, with `text` typed in it.
fn noting(text: &str) -> (AppState, Ui) {
    let app = opened();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    ui.session.noting = Some(Noting::new(Anchor::line("src/lib.rs", 1)));
    ui.focus = crate::Focus::Workspace;
    typed(text, &mut ui, &app);
    (app, ui)
}

fn said(ui: &Ui) -> String {
    ui.session
        .noting
        .as_ref()
        .map(|one| one.buffer.text())
        .unwrap_or_default()
}

#[test]
fn the_caret_moves_across_the_lines_and_types_where_it_stands() {
    let (app, mut ui) = noting("ab");
    chord(Key::Enter, shift(), &mut ui, &app);
    typed("cd", &mut ui, &app);
    press(Key::Up, &mut ui, &app);
    typed("X", &mut ui, &app);
    assert_eq!(said(&ui), "abX\ncd", "up keeps the column");
    press(Key::Down, &mut ui, &app);
    press(Key::Home, &mut ui, &app);
    press(Key::Delete, &mut ui, &app);
    press(Key::End, &mut ui, &app);
    typed("!", &mut ui, &app);
    assert_eq!(said(&ui), "abX\nd!");
    press(Key::Left, &mut ui, &app);
    press(Key::Backspace, &mut ui, &app);
    assert_eq!(said(&ui), "abX\n!");
}

#[test]
fn a_selection_cuts_to_the_clipboard_and_an_undo_brings_it_back() {
    let (app, mut ui) = noting("issue: leaks");
    for _ in 0..5 {
        chord(Key::Left, shift(), &mut ui, &app);
    }
    let cut = chord(Key::Char('x'), ctrl(), &mut ui, &app);
    let copied = Command::Workspace(workspace::Command::CopyText {
        text: "leaks".into(),
    });
    assert_eq!(cut, [copied]);
    assert_eq!(said(&ui), "issue: ");
    chord(Key::Char('z'), ctrl(), &mut ui, &app);
    assert_eq!(said(&ui), "issue: leaks", "undo puts it back");
    chord(Key::Char('y'), ctrl(), &mut ui, &app);
    assert_eq!(said(&ui), "issue: ", "and redo takes it again");
    chord(Key::Char('z'), ctrl(), &mut ui, &app);

    chord(Key::Char('a'), ctrl(), &mut ui, &app);
    let all = chord(Key::Char('c'), ctrl(), &mut ui, &app);
    let whole = Command::Workspace(workspace::Command::CopyText {
        text: "issue: leaks".into(),
    });
    assert_eq!(all, [whole], "select all, then copy");
    assert_eq!(said(&ui), "issue: leaks", "a copy leaves the words");
}

#[test]
fn a_paste_lands_at_the_caret_with_its_lines() {
    let (app, mut ui) = noting("ab");
    press(Key::Left, &mut ui, &app);
    let paste = Input::Paste("one\r\ntwo\n".into());
    handle(paste, &mut ui, &app, &crate::hit::Hits::default(), window());
    assert_eq!(said(&ui), "aone\ntwo\nb", "its final break too");
}

#[test]
fn the_row_draws_the_caret_and_what_is_held_the_way_the_commit_box_does() {
    let (app, mut ui) = noting("leaks");
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let count = |ui: &Ui, color: groove_gfx::Color| {
        let (frame, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        let code = hits.rect_of(&Target::Code).expect("the rows");
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == color && code.contains(quad.rect.x, quad.rect.y))
            .count()
    };
    assert_eq!(count(&ui, styles.caret()), 1, "the note holds the caret");
    assert!(
        !row_texts(&app, &ui)
            .iter()
            .any(|one| one.contains('\u{2502}'))
    );
    let held = count(&ui, styles.held());
    chord(Key::Home, shift(), &mut ui, &app);
    assert_eq!(
        count(&ui, styles.held()),
        held + 1,
        "the words held are shaded"
    );
}

#[test]
fn ctrl_moves_the_caret_and_deletes_in_a_note_as_the_key_alone_does() {
    let (app, mut ui) = noting("abc");
    chord(Key::Left, ctrl(), &mut ui, &app);
    chord(Key::Backspace, ctrl(), &mut ui, &app);
    assert_eq!(said(&ui), "ac");
    chord(Key::Home, ctrl(), &mut ui, &app);
    chord(Key::Delete, ctrl(), &mut ui, &app);
    chord(Key::End, ctrl(), &mut ui, &app);
    typed("!", &mut ui, &app);
    chord(Key::Home, ctrl(), &mut ui, &app);
    chord(Key::Right, ctrl(), &mut ui, &app);
    typed("?", &mut ui, &app);
    assert_eq!(said(&ui), "c?!");
}

#[test]
fn a_note_on_a_tab_not_shown_takes_no_keys_and_keeps_its_words() {
    let (app, mut ui) = noting("half");
    ui.session.tab = crate::views::session::Tab::Overview;
    typed("x", &mut ui, &app);
    let posted = chord(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert!(
        !posted.iter().any(|one| matches!(one, Command::Delivery(_))),
        "{posted:?}"
    );
    ui.session.tab = crate::views::session::Tab::Files;
    assert_eq!(said(&ui), "half", "the words wait for the tab");
}

#[test]
fn a_note_on_another_file_than_the_open_one_takes_no_keys() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    ui.session.noting = Some(Noting::new(Anchor::line("src/other.rs", 0)));
    ui.focus = crate::Focus::Workspace;
    typed("x", &mut ui, &app);
    let posted = chord(Key::Enter, Modifiers::default(), &mut ui, &app);
    assert!(
        !posted.iter().any(|one| matches!(one, Command::Delivery(_))),
        "{posted:?}"
    );
    assert_eq!(said(&ui), "", "the note is not drawn");
}

#[test]
fn a_click_on_another_pane_takes_the_keys_from_a_note_and_keeps_its_words() {
    let app = opened();
    let mut ui = on_diff();
    ui.session.tab = crate::views::session::Tab::Files;
    ui.session.noting = Some(Noting::new(Anchor::line("src/lib.rs", 1)));
    ui.focus = crate::Focus::Workspace;
    typed("half", &mut ui, &app);
    for (pane, focus) in [
        (Target::Agent, crate::Focus::Agent),
        (
            Target::Session(groove_types::SessionId::new("a")),
            crate::Focus::Rail,
        ),
    ] {
        let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
        click(hits.rect_of(&pane).expect("the pane"), &mut ui, &app, &hits);
        assert_eq!(ui.focus, focus);
        let posted = press(Key::Enter, &mut ui, &app);
        assert!(
            !posted
                .iter()
                .any(|one| matches!(one, groove_controllers::Command::Delivery(_))),
            "{focus:?}: Enter posts no note: {posted:?}"
        );
    }
    let said = |ui: &Ui| ui.session.noting.as_ref().map(|one| one.buffer.text());
    assert_eq!(said(&ui).as_deref(), Some("half"), "the draft is kept");

    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    click(
        hits.rect_of(&Target::Code).expect("the rows"),
        &mut ui,
        &app,
        &hits,
    );
    typed("!", &mut ui, &app);
    assert_eq!(
        said(&ui).as_deref(),
        Some("half!"),
        "and takes the keys back"
    );
}
