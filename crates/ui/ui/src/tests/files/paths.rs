//! The worktree's own files: what the tree offers, and what a typed name asks for.

use groove_controllers::workspace_service::PathOp;
use groove_controllers::{Command, workspace};

use super::explorer::browsing;
use super::*;
use crate::input::{Input, Modifiers};
use crate::views::session::Asked;
use crate::{Losing, Of};

/// The menu a right click on this row opens.
fn menu(app: &AppState, ui: &mut Ui, target: &Target) -> Of {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let rect = hits
        .rect_of(target)
        .unwrap_or_else(|| panic!("{target:?} was drawn"));
    crate::input::handle(
        Input::Menu {
            x: rect.x + rect.w / 2.0,
            y: rect.y + rect.h / 2.0,
        },
        ui,
        app,
        &hits,
        window(),
    );
    ui.menu().expect("a menu is open").of.clone()
}

/// Picks the row named `label` of the open menu.
fn pick(app: &AppState, ui: &mut Ui, label: &str) -> Vec<Command> {
    let of = ui.menu().expect("a menu").of.clone();
    let at = crate::views::overlays::actions::rows(&of)
        .iter()
        .position(|row| *row == label)
        .unwrap_or_else(|| panic!("{label} is offered"));
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    let rect = hits
        .rect_of(&Target::MenuRow(at))
        .expect("the row was drawn");
    crate::input::handle(
        Input::Press {
            x: rect.x + rect.w / 2.0,
            y: rect.y + rect.h / 2.0,
            mods: Default::default(),
        },
        ui,
        app,
        &hits,
        window(),
    )
}

fn typed(text: &str, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let mut last = Vec::new();
    for c in text.chars() {
        let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
        last = crate::input::handle(
            Input::Key {
                key: Key::Char(c),
                mods: Modifiers::default(),
            },
            ui,
            app,
            &hits,
            window(),
        );
    }
    last
}

fn enter(ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let (_, hits) = view(app, ui, window(), &mut Fonts::embedded());
    crate::input::handle(
        Input::Key {
            key: Key::Enter,
            mods: Modifiers::default(),
        },
        ui,
        app,
        &hits,
        window(),
    )
}

#[test]
fn a_directory_offers_what_can_be_done_to_a_path() {
    let (app, mut ui) = browsing(&["src/one/alpha.rs"]);
    let of = menu(&app, &mut ui, &Target::Dir("src".into()));
    assert_eq!(
        of,
        Of::Path {
            path: "src".into(),
            dir: true
        }
    );
    let rows = crate::views::overlays::actions::rows(&of);
    assert_eq!(
        rows,
        ["new file", "new directory", "rename", "copy", "delete"]
    );
}

#[test]
fn a_new_file_is_named_inside_the_directory_it_was_asked_of() {
    let (app, mut ui) = browsing(&["src/one/alpha.rs"]);
    menu(&app, &mut ui, &Target::Dir("src".into()));
    assert!(pick(&app, &mut ui, "new file").is_empty(), "it asks first");
    let naming = ui.session.naming.clone().expect("a name is asked for");
    assert_eq!(naming.asked, Asked::File);
    assert_eq!(naming.at, "src");
    assert!(naming.named().is_empty(), "and nothing is prefilled");

    typed("two.rs", &mut ui, &app);
    let asked = enter(&mut ui, &app);
    assert_eq!(
        asked,
        vec![Command::Workspace(workspace::Command::Path(
            PathOp::Create {
                path: "src/two.rs".into(),
                folder: false
            }
        ))]
    );
    assert!(ui.session.naming.is_none(), "and the field is spent");
}

#[test]
fn the_name_is_typed_where_it_will_stand() {
    let (app, mut ui) = browsing(&["src/one/alpha.rs"]);
    ui.session.opened.insert("src".into());
    menu(&app, &mut ui, &Target::Dir("src".into()));
    pick(&app, &mut ui, "new file");
    typed("two", &mut ui, &app);
    let rows = crate::views::session::files::explorer::rows(&app.workspace.paths, &[], &ui);
    let at = rows
        .iter()
        .position(|row| row.path.is_empty())
        .expect("the row being named");
    assert_eq!(rows[at - 1].path, "src", "just inside the directory");
    assert_eq!(rows[at].depth, 1, "and one level into it");
}

#[test]
fn renaming_a_file_starts_from_the_name_it_has() {
    let (app, mut ui) = browsing(&["src/one/alpha.rs"]);
    ui.session.opened.insert("src".into());
    ui.session.opened.insert("src/one".into());
    menu(&app, &mut ui, &Target::File("src/one/alpha.rs".into()));
    pick(&app, &mut ui, "rename");
    let naming = ui.session.naming.clone().expect("a name");
    assert_eq!(naming.asked, Asked::Rename);
    assert_eq!(naming.named(), "alpha.rs", "the name it has");

    typed("2", &mut ui, &app);
    let asked = enter(&mut ui, &app);
    assert_eq!(
        asked,
        vec![Command::Workspace(workspace::Command::Path(
            PathOp::Rename {
                from: "src/one/alpha.rs".into(),
                to: "src/one/alpha.rs2".into()
            }
        ))],
        "it moves beside itself"
    );
}

#[test]
fn escape_takes_the_name_back_and_asks_nothing() {
    let (app, mut ui) = browsing(&["src/one/alpha.rs"]);
    menu(&app, &mut ui, &Target::Dir("src".into()));
    pick(&app, &mut ui, "new directory");
    typed("deep", &mut ui, &app);
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let asked = crate::input::handle(
        Input::Key {
            key: Key::Escape,
            mods: Modifiers::default(),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert!(asked.is_empty());
    assert!(ui.session.naming.is_none());
}

#[test]
fn an_empty_name_asks_nothing_of_the_worktree() {
    let (app, mut ui) = browsing(&["src/one/alpha.rs"]);
    menu(&app, &mut ui, &Target::Dir("src".into()));
    pick(&app, &mut ui, "new file");
    assert!(enter(&mut ui, &app).is_empty(), "nothing is made");
    assert!(ui.session.naming.is_none());
}

#[test]
fn deleting_a_path_asks_in_its_own_row_first() {
    let (app, mut ui) = browsing(&["src/one/alpha.rs"]);
    menu(&app, &mut ui, &Target::Dir("src".into()));
    let asked = pick(&app, &mut ui, "delete");
    assert!(asked.is_empty(), "it asks before it deletes");
    assert_eq!(ui.losing(), Some(&Losing::Path("src".into())));

    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|one| one.text.clone())
        .collect();
    assert!(texts.iter().any(|one| one == "delete it?"), "{texts:?}");
    let rect = hits.rect_of(&Target::Discard).expect("the answer");
    let done = crate::input::handle(
        Input::Press {
            x: rect.x + rect.w / 2.0,
            y: rect.y + rect.h / 2.0,
            mods: Default::default(),
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    assert_eq!(
        done,
        vec![Command::Workspace(workspace::Command::Path(
            PathOp::Delete { path: "src".into() }
        ))]
    );
}
