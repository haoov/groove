//! The completion list: drawn under the caret, chosen from with the arrows, typed with tab.

use groove_controllers::{AppState, Command, workspace};
use groove_gfx::Fonts;
use groove_types::{Caret, Completion, Edit, Motion};

use super::diagnostics::{PATH, diagnosed};
use crate::input::{Key, Modifiers};
use crate::tests::{metrics, press, set_face};
use crate::views::session::Face;
use crate::{Focus, Ui, view};

/// After `No` of `x = None`.
const CARET: Caret = Caret { line: 0, column: 6 };

fn item(label: &str) -> Completion {
    Completion {
        label: label.into(),
        detail: Some(format!("{label} detail")),
        insert: label.into(),
        range: None,
        filter: label.into(),
        sort: label.into(),
    }
}

/// The open file with its caret after `No`, and the servers' offer standing there.
fn offered() -> AppState {
    let mut app = diagnosed(groove_types::Severity::Error);
    app.workspace.edit(&Edit::Move(Motion::To(CARET)));
    let asked = app.workspace.servers.offering(PATH, CARET);
    let items = ["None", "NotImplemented", "abs"].map(item).to_vec();
    app.workspace.servers.offers(asked, items);
    app
}

fn on_file() -> Ui {
    let mut ui = Ui {
        focus: Focus::Workspace,
        ..Ui::default()
    };
    set_face(&mut ui, Face::File);
    ui
}

/// The texts of the frame's top layer.
fn listed(app: &AppState, ui: &Ui) -> Vec<String> {
    let (frame, _) = view(app, ui, metrics(1600, 1000, 1.0), &mut Fonts::embedded());
    let top = frame.layers().last().expect("a layer");
    top.texts.iter().map(|run| run.text.clone()).collect()
}

fn edit(one: Edit) -> Command {
    Command::Workspace(workspace::Command::Edit(one))
}

#[test]
fn the_list_under_the_caret_holds_what_matches_the_typed_word() {
    let app = offered();
    let shown = listed(&app, &on_file());
    assert!(shown.contains(&"None".to_string()), "{shown:?}");
    assert!(
        shown.contains(&"NotImplemented detail".to_string()),
        "{shown:?}"
    );
    assert!(!shown.iter().any(|one| one.starts_with("abs")), "{shown:?}");
}

#[test]
fn the_list_stands_only_while_the_caret_does() {
    let mut app = offered();
    app.workspace.edit(&Edit::Move(Motion::Left));
    let shown = listed(&app, &on_file());
    assert!(
        !shown.contains(&"NotImplemented detail".to_string()),
        "{shown:?}"
    );
}

#[test]
fn down_chooses_the_next_and_tab_types_it_over_the_word() {
    let app = offered();
    let mut ui = on_file();
    let none = Modifiers::default();
    assert!(press(Key::Down, none, &mut ui, &app).is_empty());
    let typed = press(Key::Tab, none, &mut ui, &app);
    let over = Edit::Extend(Motion::To(Caret::new(0, 4)));
    assert_eq!(
        typed,
        vec![edit(over), edit(Edit::Insert("NotImplemented".into()))]
    );
}

#[test]
fn escape_puts_the_list_away_and_enter_stays_a_newline() {
    let app = offered();
    let mut ui = on_file();
    let none = Modifiers::default();
    let dismissed = press(Key::Escape, none, &mut ui, &app);
    assert_eq!(
        dismissed,
        vec![Command::Workspace(workspace::Command::Dismiss)]
    );
    assert_eq!(
        press(Key::Enter, none, &mut ui, &app),
        vec![edit(Edit::Newline)]
    );
}

#[test]
fn ctrl_space_asks_what_the_servers_offer() {
    let app = diagnosed(groove_types::Severity::Error);
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::default()
    };
    assert_eq!(
        press(Key::Char(' '), ctrl, &mut on_file(), &app),
        vec![Command::Workspace(workspace::Command::Complete)]
    );
}

#[test]
fn an_item_that_names_its_range_replaces_it_past_the_caret_too() {
    let mut app = offered();
    let asked = app.workspace.servers.offering(PATH, CARET);
    let mut whole = item("None");
    whole.range = Some((Caret::new(0, 4), Caret::new(0, 8)));
    app.workspace.servers.offers(asked, vec![whole]);
    let typed = press(Key::Tab, Modifiers::default(), &mut on_file(), &app);
    let edits = [
        Edit::Move(Motion::To(Caret::new(0, 8))),
        Edit::Extend(Motion::To(Caret::new(0, 4))),
        Edit::Insert("None".into()),
    ];
    assert_eq!(typed, edits.map(edit).to_vec());
}
