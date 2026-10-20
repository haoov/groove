//! The buttons on a note of the user's own.

use super::*;

#[test]
fn resolve_asks_for_the_note_to_be_resolved() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    let commands = on_button(&app, &mut ui, crate::hit::NoteButton::Resolve);
    assert_eq!(
        one_command(commands),
        delivery::Command::Note(groove_controllers::delivery::NoteAct::Resolve {
            id: groove_types::AnnotationId::new("n1")
        })
    );
}

#[test]
fn a_resolved_note_offers_to_open_again() {
    let mut app = opened();
    let mut note = own();
    note.resolved = true;
    app.delivery.shown = vec![note];
    let mut ui = on_diff();
    on_button(&app, &mut ui, crate::hit::NoteButton::Fold);
    let commands = on_button(&app, &mut ui, crate::hit::NoteButton::Resolve);
    assert_eq!(
        one_command(commands),
        delivery::Command::Note(groove_controllers::delivery::NoteAct::Reopen {
            id: groove_types::AnnotationId::new("n1")
        })
    );
}

#[test]
fn delete_asks_for_the_note_to_go() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    let commands = on_button(&app, &mut ui, crate::hit::NoteButton::Delete);
    assert_eq!(
        one_command(commands),
        delivery::Command::Note(groove_controllers::delivery::NoteAct::Delete {
            id: groove_types::AnnotationId::new("n1")
        })
    );
}

#[test]
fn edit_opens_the_note_with_its_own_words_in_it() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    let commands = on_button(&app, &mut ui, crate::hit::NoteButton::Edit);
    assert!(commands.is_empty(), "nothing is asked of the app yet");
    let noting = ui.session.noting.as_ref().expect("the row opens");
    assert_eq!(noting.said(), "issue: this leaks");
    assert_eq!(noting.anchor, Anchor::line("src/lib.rs", 1));
}

#[test]
fn what_is_typed_over_a_note_writes_that_note_again() {
    let mut app = opened();
    app.delivery.shown = vec![own()];
    let mut ui = on_diff();
    on_button(&app, &mut ui, crate::hit::NoteButton::Edit);
    if let Some(noting) = ui.session.noting.as_mut() {
        noting.buffer = groove_controllers::workspace_service::Buffer::plain("nitpick: name it");
    }
    let commands = press(Key::Enter, &mut ui, &app);
    assert_eq!(
        one_command(commands),
        delivery::Command::Note(groove_controllers::delivery::NoteAct::Update {
            id: groove_types::AnnotationId::new("n1"),
            content: "nitpick: name it".into()
        })
    );
}
