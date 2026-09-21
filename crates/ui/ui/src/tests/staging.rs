//! What a file's row offers: the index, and the question before a change is lost.

use groove_controllers::{AppState, Command, workspace};
use groove_gfx::Fonts;
use groove_types::{FileDiff, FileStatus};

use crate::hit::Target;
use crate::input::{Input, handle};
use crate::tests::{full_app, window};
use crate::views::session::Tab;
use crate::{Ui, view};

fn file(path: &str, staged: bool) -> FileDiff {
    FileDiff {
        path: path.into(),
        added: 2,
        deleted: 1,
        status: FileStatus::Modified,
        staged: Some(staged),
    }
}

fn listed(files: &[FileDiff]) -> AppState {
    let mut app = full_app();
    let worktree = app
        .session
        .selected()
        .and_then(|open| open.selected_worktree())
        .map(|w| w.id.clone())
        .expect("the fixture has a worktree");
    app.workspace
        .loaded(worktree, files.to_vec(), Default::default());
    app
}

fn sidebar() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::File;
    ui
}

/// The targets the sidebar drew, with the pointer wherever `hover` says.
fn drawn(app: &AppState, ui: &Ui) -> crate::hit::Hits {
    view(app, ui, window(), &mut Fonts::embedded()).1
}

/// A click in the middle of a target the last frame drew.
fn hit(target: &Target, ui: &mut Ui, app: &AppState) -> Vec<Command> {
    let hits = drawn(app, ui);
    let rect = hits
        .rect_of(target)
        .unwrap_or_else(|| panic!("{target:?} was drawn"));
    handle(
        Input::Press {
            x: rect.x + rect.w / 2.0,
            y: rect.y + rect.h / 2.0,
        },
        ui,
        app,
        &hits,
        window(),
    )
}

#[test]
fn a_row_offers_the_index_only_while_the_pointer_is_on_it() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let stage = Target::Stage("a.txt".into());
    assert!(
        drawn(&app, &ui).rect_of(&stage).is_none(),
        "nothing is offered to a pointer that is elsewhere"
    );
    ui.hover = Some(Target::File("a.txt".into()));
    assert!(
        drawn(&app, &ui).rect_of(&stage).is_some(),
        "and stage appears on the row the pointer is on"
    );
}

#[test]
fn a_staged_row_offers_the_way_back_out() {
    let app = listed(&[file("a.txt", true)]);
    let mut ui = sidebar();
    ui.hover = Some(Target::File("a.txt".into()));
    let hits = drawn(&app, &ui);
    assert!(hits.rect_of(&Target::Unstage("a.txt".into())).is_some());
    assert!(hits.rect_of(&Target::Stage("a.txt".into())).is_none());
}

#[test]
fn the_offer_stays_while_the_pointer_is_on_the_offer_itself() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.hover = Some(Target::Stage("a.txt".into()));
    assert!(
        drawn(&app, &ui)
            .rect_of(&Target::Stage("a.txt".into()))
            .is_some(),
        "or it would go as the pointer reached it"
    );
}

#[test]
fn clicking_the_offer_asks_for_the_index() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.hover = Some(Target::File("a.txt".into()));
    let asked = hit(&Target::Stage("a.txt".into()), &mut ui, &app);
    assert_eq!(
        asked,
        [Command::Workspace(workspace::Command::Stage {
            path: "a.txt".into()
        })]
    );
}

#[test]
fn the_right_button_opens_a_row_s_actions_and_a_click_closes_them() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let hits = drawn(&app, &ui);
    let row = hits
        .rect_of(&Target::File("a.txt".into()))
        .expect("a row is drawn");
    let point = (row.x + row.w / 2.0, row.y + row.h / 2.0);
    handle(
        Input::Menu {
            x: point.0,
            y: point.1,
        },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    let menu = ui.menu.clone().expect("the actions are open");
    assert_eq!(menu.of, crate::Of::File("a.txt".into()));
    assert!(
        drawn(&app, &ui).rect_of(&Target::MenuRow(0)).is_some(),
        "and drawn"
    );

    let taken = hit(&Target::MenuRow(0), &mut ui, &app);
    assert!(taken.is_empty(), "discard is not done on the spot");
    assert!(ui.menu.is_none(), "the actions close");
    assert_eq!(
        ui.discarding,
        Some(crate::Losing::File("a.txt".into())),
        "the row is asking instead"
    );
}

#[test]
fn the_row_asks_before_a_change_is_thrown_away() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.discarding = Some(crate::Losing::File("a.txt".into()));
    let hits = drawn(&app, &ui);
    assert!(
        hits.rect_of(&Target::File("a.txt".into())).is_none(),
        "the row is the question now, not the file"
    );
    assert!(hits.rect_of(&Target::Keep).is_some());

    let done = hit(&Target::Discard, &mut ui, &app);
    assert_eq!(
        done,
        [Command::Workspace(workspace::Command::Discard {
            path: "a.txt".into()
        })]
    );
    assert!(ui.discarding.is_none(), "and the question is answered");
}

#[test]
fn keeping_the_change_asks_nothing_of_git() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.discarding = Some(crate::Losing::File("a.txt".into()));
    let kept = hit(&Target::Keep, &mut ui, &app);
    assert!(kept.is_empty(), "nothing is asked");
    assert!(ui.discarding.is_none());
}

#[test]
fn a_row_under_the_pointer_and_the_open_one_read_apart() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let styles = crate::style::Styles::new(app.config.theme(), crate::tokens::Tokens::new(1.0));
    let row = crate::tokens::Tokens::new(1.0).row;
    let grounds = |ui: &Ui| {
        let (frame, _) = view(&app, ui, window(), &mut Fonts::embedded());
        let quads = frame.layers()[0].quads.clone();
        let of = |color| {
            quads
                .iter()
                .filter(|quad| quad.color == color && quad.rect.h == row)
                .count()
        };
        (of(styles.hover()), of(styles.here()))
    };
    assert_eq!(grounds(&ui), (0, 0), "a row at rest carries neither");
    ui.hover = Some(Target::File("a.txt".into()));
    assert_eq!(grounds(&ui).0, 1, "the pointer grounds the row");
    assert_ne!(styles.hover(), styles.here(), "and the two never match");
    assert_ne!(
        styles.hover(),
        styles.action(),
        "nor does a button's ground"
    );
}

#[test]
fn the_offer_takes_a_ground_only_once_the_pointer_is_on_it() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let styles = crate::style::Styles::new(app.config.theme(), crate::tokens::Tokens::new(1.0));
    let lit = |ui: &Ui| {
        let (frame, _) = view(&app, ui, window(), &mut Fonts::embedded());
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.action())
            .count()
    };
    ui.hover = Some(Target::File("a.txt".into()));
    assert_eq!(lit(&ui), 0, "the word is there, with no ground of its own");
    ui.hover = Some(Target::Stage("a.txt".into()));
    assert_eq!(lit(&ui), 1, "and it lights up under the pointer");
}
