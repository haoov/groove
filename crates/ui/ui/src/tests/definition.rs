//! Where a symbol is defined: asked by a ctrl+click, landed on in the middle of the view,
//! or said at the spot when it cannot open.

use groove_controllers::{Command, workspace};
use groove_gfx::Fonts;
use groove_types::{Caret, Selection};

use super::diagnostics::{PATH, diagnosed};
use crate::hit::Target;
use crate::input::{Input, Modifiers, handle, land};
use crate::tests::{all_texts, full_app, set_face, shows, window};
use crate::views::session::Face;
use crate::views::session::diff::Pointed;
use crate::{Focus, Ui, view};

const CTRL: Modifiers = Modifiers {
    ctrl: true,
    shift: false,
    alt: false,
};

fn on_file() -> Ui {
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    set_face(&mut ui, Face::File);
    ui
}

#[test]
fn ctrl_click_asks_where_the_symbol_under_the_pointer_is_defined() {
    let app = diagnosed(groove_types::Severity::Error);
    let mut ui = on_file();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let rows = hits.rect_of(&Target::Code).expect("the rows");
    let chars = hits.chars();
    let press = Input::Press {
        x: chars.left + 4.0 * chars.advance,
        y: rows.y + 1.0,
        mods: CTRL,
    };
    let asked = handle(press, &mut ui, &app, &hits, window());
    let defined = Command::Workspace(workspace::Command::Definition {
        path: PATH.into(),
        at: Caret::new(0, 4),
    });
    assert_eq!(asked, vec![defined]);
}

#[test]
fn a_definition_that_cannot_open_is_said_where_the_pointer_stands() {
    let mut app = diagnosed(groove_types::Severity::Error);
    let at = Caret::new(0, 4);
    let said = "Defined outside the worktree, in /lib.py. Groove opens worktree paths only.";
    app.workspace.servers.told(PATH, at, said.into());
    let mut ui = on_file();
    ui.session.pointed = Some(Pointed {
        path: PATH.into(),
        at,
    });
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        all_texts(&frame)
            .iter()
            .any(|one| one.contains("outside the worktree"))
    );
    ui.session.pointed = None;
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        !all_texts(&frame)
            .iter()
            .any(|one| one.contains("outside the worktree")),
        "gone with the pointer"
    );
}

#[test]
fn landing_on_a_definition_centres_it_once() {
    let mut app = full_app();
    let text: String = (0..300).map(|n| format!("line {n}\n")).collect();
    shows(&mut app, PATH, &text, &text);
    let at = Caret::new(200, 0);
    app.workspace.focus(PATH, Some(Selection::at(at)));
    app.workspace.servers.landed(PATH, at);
    let mut ui = on_file();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    land(&mut ui, &app, &hits, window());
    let rows = hits.rect_of(&Target::Code).expect("the rows");
    let line = window().tokens().line;
    let middle = ui.session.scroll() + rows.h / 2.0;
    assert!(
        (middle - (200.0 * line + line / 2.0)).abs() <= line,
        "centred: {middle}"
    );
    *ui.session.scroll_mut() = 0.0;
    land(&mut ui, &app, &hits, window());
    assert_eq!(ui.session.scroll(), 0.0, "once a landing");
}
