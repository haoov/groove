//! The popup: asked once the pointer rests on a spot, or from the caret on `ctrl+k`.

use groove_controllers::{AppState, Command, workspace};
use groove_gfx::Fonts;
use groove_types::Caret;

use super::diagnostics::{PATH, diagnosed};
use crate::input::{Key, Modifiers, hovering};
use crate::tests::{all_texts, metrics, press, set_face};
use crate::views::session::Face;
use crate::views::session::diff::Pointed;
use crate::{Focus, Ui, view};

const ON: Caret = Caret { line: 0, column: 5 };

fn pointing_at(at: Caret) -> Ui {
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    set_face(&mut ui, Face::File);
    ui.session.pointed = Some(Pointed {
        path: PATH.into(),
        at,
    });
    ui
}

/// What the frame says at `tick`, every layer.
fn said(app: &AppState, ui: &Ui, tick: u64) -> String {
    let mut at = metrics(1600, 1000, 1.0);
    at.tick = tick;
    let (frame, _) = view(app, ui, at, &mut Fonts::embedded());
    all_texts(&frame).join("\n")
}

#[test]
fn the_pointer_asks_once_it_has_rested_and_never_twice() {
    let mut app = diagnosed(groove_types::Severity::Error);
    let mut ui = pointing_at(ON);
    assert_eq!(hovering(&mut ui, &app, 1000), None, "it comes to rest");
    assert_eq!(hovering(&mut ui, &app, 1100), None, "not yet");
    let asked = hovering(&mut ui, &app, 1250);
    let hover = Command::Workspace(workspace::Command::Hover {
        path: PATH.into(),
        at: ON,
        caret: false,
    });
    assert_eq!(asked, Some(hover));
    app.workspace.servers.asked(PATH, ON, false);
    assert_eq!(hovering(&mut ui, &app, 1400), None, "asked already");
}

#[test]
fn a_rested_pointer_shows_the_diagnostic_over_what_the_hover_said() {
    let mut app = diagnosed(groove_types::Severity::Error);
    app.workspace.servers.asked(PATH, ON, false);
    let markdown = "```python\nNone\n```\nThe absence of a value.";
    app.workspace
        .servers
        .answered(PATH, ON, Some(markdown.into()));
    let mut ui = pointing_at(ON);
    ui.session.pointer_rest = ui.session.pointed.clone().map(|at| (at, 0));
    let shown = said(&app, &ui, 300);
    let message = shown.find("expected `int`").expect("the diagnostic");
    let hover = shown.find("The absence of a value.").expect("the hover");
    assert!(message < hover, "the diagnostic stands above: {shown}");
    assert!(shown.contains("None"), "the code block, its fence dropped");
    assert!(!shown.contains("```"), "{shown}");
    assert!(shown.contains("ty"), "who found it");
}

#[test]
fn the_popup_waits_for_the_rest_and_leaves_with_the_pointer() {
    let mut app = diagnosed(groove_types::Severity::Error);
    app.workspace.servers.asked(PATH, ON, false);
    let mut ui = pointing_at(ON);
    ui.session.pointer_rest = ui.session.pointed.clone().map(|at| (at, 1000));
    assert!(
        !said(&app, &ui, 1100).contains("expected `int`"),
        "too soon"
    );
    assert!(said(&app, &ui, 1300).contains("expected `int`"));
    ui.session.pointed = Some(Pointed {
        path: PATH.into(),
        at: Caret::new(0, 0),
    });
    assert!(
        !said(&app, &ui, 1400).contains("expected `int`"),
        "moved off"
    );
}

#[test]
fn ctrl_k_asks_at_the_caret() {
    let app = diagnosed(groove_types::Severity::Error);
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    set_face(&mut ui, Face::File);
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    let asked = press(Key::Char('k'), ctrl, &mut ui, &app);
    let caret = app.workspace.active().expect("open").new.caret();
    let hover = Command::Workspace(workspace::Command::Hover {
        path: PATH.into(),
        at: caret,
        caret: true,
    });
    assert_eq!(asked, vec![hover]);
}

#[test]
fn a_popup_asked_at_the_caret_stands_while_the_caret_does() {
    let mut app = diagnosed(groove_types::Severity::Error);
    let caret = app.workspace.active().expect("open").new.caret();
    app.workspace
        .servers
        .answered(PATH, caret, Some("unused".into()));
    app.workspace.servers.asked(PATH, caret, true);
    app.workspace
        .servers
        .answered(PATH, caret, Some("x: int".into()));
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    set_face(&mut ui, Face::File);
    assert!(said(&app, &ui, 0).contains("x: int"), "at once, no rest");
    ui.focus = Focus::Rail;
    assert!(!said(&app, &ui, 0).contains("x: int"), "the keyboard left");
}
