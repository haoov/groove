//! A ctrl+click lands on the definition in the middle of the view, or says why at the spot.

use groove_controllers::{Command, workspace};
use groove_gfx::Fonts;
use groove_types::{Caret, Selection};

use super::diagnostics::{PATH, diagnosed};
use super::hover::moved_to;
use crate::hit::Target;
use crate::input::{Input, Modifiers, handle, land};
use crate::tests::{all_texts, full_app, on_file, shows, window};
use crate::view;

const CTRL: Modifiers = Modifiers {
    ctrl: true,
    shift: false,
    alt: false,
};

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
    moved_to(&mut ui, &app, at, window());
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    assert!(
        all_texts(&frame)
            .iter()
            .any(|one| one.contains("outside the worktree"))
    );
    moved_to(&mut ui, &app, Caret::new(0, 0), window());
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
    let line = window().tokens().code_line;
    let middle = ui.session.scroll() + rows.h / 2.0;
    assert!(
        (middle - (200.0 * line + line / 2.0)).abs() <= line,
        "centred: {middle}"
    );
    *ui.session.scroll_mut() = 0.0;
    land(&mut ui, &app, &hits, window());
    assert_eq!(ui.session.scroll(), 0.0, "once a landing");
}

#[test]
fn a_definition_the_diff_does_not_show_opens_on_the_file_tab() {
    use crate::tests::changed_files;
    use crate::views::session::Tab;
    let mut app = full_app();
    shows(&mut app, "src/model.py", "class Quote:\n", "class Quote:\n");
    changed_files(&mut app, &[("src/cache.py", "a\n", "b\n")]);
    let at = Caret::new(0, 6);
    app.workspace.focus("src/model.py", Some(Selection::at(at)));
    app.workspace.servers.landed("src/model.py", at);
    let mut ui = on_file();
    ui.session.tab = Tab::Diff;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    land(&mut ui, &app, &hits, window());
    assert_eq!(ui.session.tab, Tab::Files);
}

#[test]
fn a_definition_the_diff_shows_stays_on_the_diff() {
    use crate::views::session::Tab;
    let mut app = full_app();
    shows(&mut app, PATH, "x = 1\n", "x = 2\n");
    let at = Caret::new(0, 0);
    app.workspace.servers.landed(PATH, at);
    let mut ui = on_file();
    ui.session.tab = Tab::Diff;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    land(&mut ui, &app, &hits, window());
    assert_eq!(ui.session.tab, Tab::Diff);
}

#[test]
fn the_palette_restarts_the_language_servers() {
    let app = diagnosed(groove_types::Severity::Error);
    let all = crate::palette::entries(&app);
    let restart = Command::Workspace(workspace::Command::RestartServers);
    let found = all
        .iter()
        .find(|one| one.label == "Restart the language servers");
    assert_eq!(
        found.map(|one| &one.run),
        Some(&crate::palette::Run::Command(restart))
    );
}
