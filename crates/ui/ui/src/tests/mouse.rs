use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{SessionId, WorktreeId};

use crate::hit::{Cursor, Hits, Target};
use crate::input::{Key, Modifiers};
use crate::layout::{Edge, Layout, Split};
use crate::tests::{WINDOW, click, drag, full_app, metrics, press, release};
use crate::tokens::{AGENT_MIN, RAIL_MIN, Tokens, WORKSPACE_MIN};
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

/// The rail's boundary, as the frame drew it.
fn rail_edge(hits: &Hits) -> groove_gfx::Rect {
    hits.rect_of(&Target::Split(Edge::Rail))
        .expect("the rail has a splitter")
}

fn columns(split: Split) -> Layout {
    let tokens = Tokens::new(1.0);
    Layout::new(groove_gfx::Size::new(WINDOW.0, WINDOW.1), &tokens, split)
}

#[test]
fn a_press_on_a_boundary_takes_hold_of_it() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let edge = rail_edge(&hits);
    assert!(click(edge, &mut ui, &app, &hits).is_empty());
    assert!(ui.dragging(), "the press grabbed the boundary");
    drag(400.0, &mut ui, &app, &hits);
    assert_eq!(ui.split.rail, 400.0);
    release(&mut ui, &app, &hits);
    assert!(!ui.dragging());
    assert_eq!(
        columns(ui.split).rail.w,
        400.0,
        "the rail follows the split"
    );
}

#[test]
fn a_drag_never_starves_a_column() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let edge = rail_edge(&hits);
    click(edge, &mut ui, &app, &hits);
    drag(0.0, &mut ui, &app, &hits);
    assert_eq!(ui.split.rail, RAIL_MIN);
    drag(WINDOW.0 as f32, &mut ui, &app, &hits);
    let left = WINDOW.0 as f32 - AGENT_MIN - WORKSPACE_MIN;
    assert_eq!(
        ui.split.rail, left,
        "the agent and the workspace keep their own"
    );
}

#[test]
fn the_agent_edge_leaves_the_workspace_its_minimum() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let edge = hits
        .rect_of(&Target::Split(Edge::Agent))
        .expect("the agent pane has a splitter");
    click(edge, &mut ui, &app, &hits);
    drag(WINDOW.0 as f32, &mut ui, &app, &hits);
    let workspace = columns(ui.split).workspace.w;
    assert_eq!(workspace, WORKSPACE_MIN);
}

#[test]
fn the_pointer_resizes_over_a_boundary_and_stays_so_while_dragging() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let edge = rail_edge(&hits);
    let (x, y) = (edge.x + 1.0, edge.y + 1.0);
    assert_eq!(crate::input::cursor(&ui, &hits, x, y), Cursor::ColResize);
    click(edge, &mut ui, &app, &hits);
    assert_eq!(
        crate::input::cursor(&ui, &hits, 0.0, 0.0),
        Cursor::ColResize,
        "a drag owns the pointer wherever it goes"
    );
}
