//! What the forge answers for: a note posted, a thread replied to or resolved.

use super::*;

/// A thread of the merge request, on line 1.
fn thread() -> groove_types::Note {
    groove_types::Note {
        origin: groove_types::NoteOrigin::Thread("t1".into()),
        anchor: Some(Anchor::line("src/lib.rs", 1)),
        resolved: false,
        said: vec![groove_types::Said {
            author: "reviewer".into(),
            body: "issue: this leaks".into(),
            at: groove_types::Timestamp::new(0),
        }],
    }
}

/// The app with an MR of its own, so a note of it can be posted.
fn delivered(notes: Vec<groove_types::Note>) -> AppState {
    let mut app = opened();
    app.workspace.notes = notes;
    app.workspace.delivery.mr = Some(groove_types::Mr {
        id: groove_types::MrId::new("m1"),
        worktree: groove_types::WorktreeId::new("wt-1"),
        forge: groove_types::Forge::Gitlab,
        remote_id: "7".into(),
        url: "https://example.com/7".into(),
        state: groove_types::MrState::Open,
    });
    app
}

/// A click on one button of one note's row.
fn on(
    app: &AppState,
    ui: &mut Ui,
    origin: groove_types::NoteOrigin,
    button: crate::hit::NoteButton,
) -> Vec<groove_controllers::Command> {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let rect = hits
        .rect_of(&Target::Note(origin, button))
        .unwrap_or_else(|| panic!("{button:?} is drawn"));
    click(rect, ui, app, &hits)
}

fn local() -> groove_types::NoteOrigin {
    groove_types::NoteOrigin::Local(groove_types::AnnotationId::new("n1"))
}

#[test]
fn a_note_of_a_session_with_an_mr_offers_to_post_it() {
    let app = delivered(vec![own()]);
    let mut ui = on_diff();
    let commands = on(&app, &mut ui, local(), crate::hit::NoteButton::Post);
    assert_eq!(
        one_command(commands),
        workspace::Command::Note(groove_controllers::workspace::NoteAct::Post {
            id: groove_types::AnnotationId::new("n1")
        })
    );
}

#[test]
fn a_session_with_no_mr_offers_no_post() {
    let mut app = opened();
    app.workspace.notes = vec![own()];
    let ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        hits.rect_of(&Target::Note(local(), crate::hit::NoteButton::Post))
            .is_none(),
        "nothing to post it on"
    );
}

#[test]
fn a_thread_offers_a_reply_and_a_resolve_and_nothing_else() {
    let app = delivered(vec![thread()]);
    let ui = on_diff();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let origin = groove_types::NoteOrigin::Thread("t1".into());
    for button in [
        crate::hit::NoteButton::Reply,
        crate::hit::NoteButton::Resolve,
    ] {
        assert!(
            hits.rect_of(&Target::Note(origin.clone(), button))
                .is_some(),
            "{button:?} stands under the thread"
        );
    }
    for button in [
        crate::hit::NoteButton::Edit,
        crate::hit::NoteButton::Delete,
        crate::hit::NoteButton::Post,
    ] {
        assert!(
            hits.rect_of(&Target::Note(origin.clone(), button))
                .is_none(),
            "{button:?} is not the forge's to offer"
        );
    }
}

#[test]
fn resolving_a_thread_asks_the_forge() {
    let app = delivered(vec![thread()]);
    let mut ui = on_diff();
    let origin = groove_types::NoteOrigin::Thread("t1".into());
    let commands = on(&app, &mut ui, origin, crate::hit::NoteButton::Resolve);
    assert_eq!(
        one_command(commands),
        workspace::Command::Note(groove_controllers::workspace::NoteAct::Thread {
            thread: "t1".into(),
            resolve: true
        })
    );
}

#[test]
fn a_resolved_thread_asks_to_be_opened_again() {
    let mut one = thread();
    one.resolved = true;
    let app = delivered(vec![one]);
    let mut ui = on_diff();
    let origin = groove_types::NoteOrigin::Thread("t1".into());
    let commands = on(&app, &mut ui, origin, crate::hit::NoteButton::Resolve);
    assert_eq!(
        one_command(commands),
        workspace::Command::Note(groove_controllers::workspace::NoteAct::Thread {
            thread: "t1".into(),
            resolve: false
        })
    );
}

#[test]
fn a_reply_is_typed_under_the_thread_it_answers() {
    let app = delivered(vec![thread()]);
    let mut ui = on_diff();
    let origin = groove_types::NoteOrigin::Thread("t1".into());
    let commands = on(&app, &mut ui, origin, crate::hit::NoteButton::Reply);
    assert!(commands.is_empty(), "nothing is asked of the forge yet");
    let noting = ui.session.noting.as_ref().expect("the row opens");
    assert_eq!(noting.anchor, Anchor::line("src/lib.rs", 1));
    assert!(noting.said().is_empty(), "with nothing in it");

    typed("fixed", &mut ui, &app);
    let commands = press(Key::Enter, &mut ui, &app);
    assert_eq!(
        one_command(commands),
        workspace::Command::Note(groove_controllers::workspace::NoteAct::Reply {
            thread: "t1".into(),
            body: "fixed".into()
        })
    );
}
