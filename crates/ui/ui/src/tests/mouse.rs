use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{SessionId, WorktreeId};

use crate::hit::{Cursor, Hits, Target};
use crate::input::{Key, Modifiers};
use crate::tests::{click, full_app, metrics, press};
use crate::views::session::Tab;
use crate::{Ui, view};

const CHORD: Modifiers = Modifiers {
    ctrl: true,
    shift: true,
    alt: false,
};

/// What the frame drew, and where.
fn regions(app: &AppState, ui: &Ui) -> Hits {
    let (_, hits) = view(app, ui, metrics(1280, 800, 1.0), &mut Fonts::embedded());
    hits
}

#[test]
fn a_click_on_a_rail_row_selects_that_session() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let rect = hits
        .rect_of(&Target::Session(SessionId::new("b")))
        .expect("Beta has a row");
    let commands = click(rect, &mut ui, &app, &hits);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id(), "session.select");
}

#[test]
fn a_click_on_a_worktree_row_selects_it() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let rect = hits
        .rect_of(&Target::Worktree(WorktreeId::new("wt-1")))
        .expect("the overview lists the worktree");
    let commands = click(rect, &mut ui, &app, &hits);
    assert_eq!(commands[0].id(), "session.select_worktree");
}

#[test]
fn a_click_on_a_picker_opens_the_worktree_selector() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let rect = hits.rect_of(&Target::Picker).expect("the header has one");
    assert!(click(rect, &mut ui, &app, &hits).is_empty());
    let prompt = ui.palette.as_ref().and_then(|p| p.prompt(&app));
    assert_eq!(prompt.map(|p| p.label), Some("worktree"));
}

#[test]
fn a_click_on_a_tab_shows_it() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let rect = hits
        .rect_of(&Target::Tab(Tab::Overview))
        .expect("the strip has one region per tab");
    assert!(click(rect, &mut ui, &app, &hits).is_empty());
    assert_eq!(ui.session.tab, Tab::Overview);
}

#[test]
fn a_click_on_a_palette_row_runs_it() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char('p'), CHORD, &mut ui, &app);
    let hits = regions(&app, &ui);
    let rect = hits
        .rect_of(&Target::PaletteRow(0))
        .expect("the palette lists its rows");
    let commands = click(rect, &mut ui, &app, &hits);
    assert_eq!(commands[0].id(), "session.open_explorer");
    assert!(ui.palette.is_none());
}

#[test]
fn a_click_outside_the_palette_closes_it() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char('p'), CHORD, &mut ui, &app);
    let hits = regions(&app, &ui);
    let rect = hits
        .rect_of(&Target::Worktree(WorktreeId::new("wt-1")))
        .expect("the overview is under the palette");
    assert!(
        click(rect, &mut ui, &app, &hits).is_empty(),
        "the click closes the palette and does nothing else"
    );
    assert!(ui.palette.is_none());
}

#[test]
fn the_pointer_says_where_a_click_lands() {
    let app = full_app();
    let ui = Ui::default();
    let hits = regions(&app, &ui);
    let row = hits
        .rect_of(&Target::Session(SessionId::new("b")))
        .expect("Beta has a row");
    assert_eq!(hits.cursor_at(row.x + 1.0, row.y + 1.0), Cursor::Pointer);
    assert_eq!(
        hits.cursor_at(row.x, row.bottom() + row.h),
        Cursor::Default,
        "nothing is drawn under the rail's rows"
    );
}
