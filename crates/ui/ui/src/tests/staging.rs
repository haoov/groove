//! What a file's row offers: the index.

use groove_controllers::{AppState, Command, workspace};
use groove_gfx::Fonts;
use groove_types::{FileDiff, FileStatus};

use crate::hit::Target;
use crate::tests::{drawn_hits, full_app, hit, window};
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
        .selected_worktree()
        .map(|w| w.id.clone())
        .expect("the fixture has a worktree");
    app.workspace
        .loaded(worktree, files.to_vec(), Default::default());
    app
}

fn sidebar() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = Tab::Diff;
    ui
}

#[test]
fn a_row_offers_the_index_only_while_the_pointer_is_on_it() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let stage = Target::Stage("a.txt".into());
    assert!(
        drawn_hits(&app, &ui).rect_of(&stage).is_none(),
        "nothing is offered to a pointer that is elsewhere"
    );
    ui.hover = Some(Target::File("a.txt".into()));
    assert!(
        drawn_hits(&app, &ui).rect_of(&stage).is_some(),
        "and stage appears on the row the pointer is on"
    );
}

#[test]
fn a_staged_row_offers_the_way_back_out() {
    let app = listed(&[file("a.txt", true)]);
    let mut ui = sidebar();
    ui.hover = Some(Target::File("a.txt".into()));
    let hits = drawn_hits(&app, &ui);
    assert!(hits.rect_of(&Target::Unstage("a.txt".into())).is_some());
    assert!(hits.rect_of(&Target::Stage("a.txt".into())).is_none());
}

#[test]
fn the_offer_stays_while_the_pointer_is_on_the_offer_itself() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    ui.hover = Some(Target::Stage("a.txt".into()));
    assert!(
        drawn_hits(&app, &ui)
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
fn a_row_under_the_pointer_and_the_open_one_read_apart() {
    let app = listed(&[file("a.txt", false)]);
    let mut ui = sidebar();
    let styles = groove_ui_kit::base::style::Styles::new(
        app.config.theme(),
        groove_ui_kit::base::tokens::Tokens::new(1.0),
    );
    let row = groove_ui_kit::base::tokens::Tokens::new(1.0).row;
    let grounds = |ui: &Ui| {
        let (frame, hits) = view(&app, ui, window(), &mut Fonts::embedded());
        let line = hits
            .rect_of(&Target::File("a.txt".into()))
            .expect("the row");
        let quads = frame.layers()[0].quads.clone();
        let of = |color| {
            quads
                .iter()
                .filter(|quad| quad.color == color && quad.rect.h > row / 2.0)
                .filter(|quad| line.contains(quad.rect.x, quad.rect.y + 1.0))
                .count()
        };
        (of(styles.glass_hover()), of(styles.glass_chosen()))
    };
    assert_eq!(grounds(&ui), (0, 0), "a row at rest carries neither");
    ui.hover = Some(Target::File("a.txt".into()));
    assert_eq!(grounds(&ui).0, 1, "the pointer grounds the row");
    assert_ne!(
        styles.glass_hover(),
        styles.glass_chosen(),
        "and the two never match"
    );
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
    let styles = groove_ui_kit::base::style::Styles::new(
        app.config.theme(),
        groove_ui_kit::base::tokens::Tokens::new(1.0),
    );
    let lit = |ui: &Ui| {
        let (frame, _) = view(&app, ui, window(), &mut Fonts::embedded());
        frame.layers()[0]
            .quads
            .iter()
            .filter(|quad| quad.color == styles.glass_hover())
            .count()
    };
    ui.hover = Some(Target::File("a.txt".into()));
    let row = lit(&ui);
    ui.hover = Some(Target::Stage("a.txt".into()));
    assert_eq!(
        lit(&ui),
        row + 1,
        "the word lights up only under the pointer"
    );
}
