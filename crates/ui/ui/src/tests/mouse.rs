use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{SessionId, WorktreeId};

use crate::base::hit::{Cursor, Hits, Picks, Target};
use crate::base::tokens::{AGENT_MIN, RAIL_MIN, Tokens, WORKSPACE_MIN};
use crate::input::Key;
use crate::layout::{Edge, Layout, Split};
use crate::tests::{
    CHORD, WINDOW, click, drag, drag_at, full_app, metrics, press, pressed, release, window,
};
use crate::views::session::Tab;
use crate::{Ui, view};

/// What the frame drew, and where.
fn regions(app: &AppState, ui: &Ui) -> Hits {
    let (_, hits) = view(app, ui, metrics(1280, 800, 1.0), &mut Fonts::embedded());
    hits
}

#[test]
fn a_click_on_a_rail_row_opens_that_session() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let rect = hits
        .rect_of(&Target::Session(SessionId::new("b")))
        .expect("Beta has a row");
    let commands = click(rect, &mut ui, &app, &hits);
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].id(), "session.open");
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
    let rect = hits
        .rect_of(&Target::Picker(Picks::Repo))
        .expect("the header has one");
    assert!(click(rect, &mut ui, &app, &hits).is_empty());
    let prompt = ui.palette().and_then(|p| p.prompt(&app));
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
    assert!(ui.palette().is_none());
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
    assert!(ui.palette().is_none());
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
    Layout::new(
        groove_gfx::Size::new(WINDOW.0, WINDOW.1),
        &tokens,
        split,
        false,
    )
}

#[test]
fn a_press_on_a_boundary_takes_hold_of_it() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let edge = rail_edge(&hits);
    let before = columns(ui.split);
    assert!(click(edge, &mut ui, &app, &hits).is_empty());
    assert!(ui.dragging(), "the press grabbed the boundary");
    drag(300.0, &mut ui, &app, &hits);
    assert_eq!(ui.split.rail, 300.0);
    release(&mut ui, &app, &hits);
    assert!(!ui.dragging());
    let after = columns(ui.split);
    assert_eq!(after.rail.w, 300.0, "the rail follows the split");
    assert_eq!(
        after.agent.right(),
        before.agent.right(),
        "a drag moves the two columns its boundary stands between"
    );
    assert_eq!(after.workspace, before.workspace, "and nothing else");
}

#[test]
fn a_drag_never_starves_a_column() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let edge = rail_edge(&hits);
    let held = ui.split.rail + ui.split.agent;
    click(edge, &mut ui, &app, &hits);
    drag(0.0, &mut ui, &app, &hits);
    assert_eq!(ui.split.rail, RAIL_MIN);
    assert_eq!(ui.split.agent, held - RAIL_MIN, "the agent takes the rest");
    drag(WINDOW.0 as f32, &mut ui, &app, &hits);
    assert_eq!(
        ui.split.agent, AGENT_MIN,
        "the agent gives what it has and no more"
    );
    assert_eq!(ui.split.rail, held - AGENT_MIN);
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

#[test]
fn the_sidebar_edge_drags_only_where_the_sidebar_stands() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = crate::views::session::Tab::File;
    let hits = regions(&app, &ui);
    let edge = hits
        .rect_of(&Target::Split(Edge::Sidebar))
        .expect("the diff tab has a sidebar to drag");
    let agent = wide(ui.split).agent.w;
    click(edge, &mut ui, &app, &hits);

    let width = WINDOW.0 as f32;
    drag(width - 340.0, &mut ui, &app, &hits);
    assert_eq!(ui.split.sidebar, 340.0);
    assert_eq!(
        wide(ui.split).agent.w,
        agent,
        "the workspace gives the room, not the agent"
    );

    drag(width - 600.0, &mut ui, &app, &hits);
    assert_eq!(
        wide(ui.split).workspace.w,
        WORKSPACE_MIN,
        "as far as it goes"
    );
    release(&mut ui, &app, &hits);
    assert!(!ui.dragging());

    let overview = Ui::default();
    let folded = regions(&app, &overview);
    assert!(
        folded.rect_of(&Target::Split(Edge::Sidebar)).is_none(),
        "no sidebar, no boundary to grab"
    );
}

/// The columns with the sidebar beside them.
fn wide(split: Split) -> Layout {
    let tokens = Tokens::new(1.0);
    Layout::new(
        groove_gfx::Size::new(WINDOW.0, WINDOW.1),
        &tokens,
        split,
        true,
    )
}

#[test]
fn the_commit_box_is_dragged_taller_and_the_list_keeps_its_room() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.tab = Tab::File;
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let band = hits
        .rect_of(&Target::Split(Edge::Commit))
        .expect("a band across the sidebar");
    assert_eq!(
        hits.cursor_at(band.x + 1.0, band.y + band.h / 2.0),
        crate::Cursor::RowResize,
        "and it says which way it moves"
    );

    let before = ui.split.commit;
    pressed(band.x + 1.0, band.y + band.h / 2.0, &mut ui, &app, &hits);
    assert!(ui.dragging());
    drag_at(band.x + 1.0, band.y - 100.0, &mut ui, &app, &hits);
    assert!(ui.split.commit > before, "taller: {}", ui.split.commit);
    release(&mut ui, &app, &hits);

    drag_at(band.x + 1.0, 0.0, &mut ui, &app, &hits);
    let layout = crate::layout::Layout::of(window(), &ui);
    assert!(
        layout.commit.y > layout.sidebar.y,
        "the list above it keeps room whatever the pointer asks"
    );
}

#[test]
fn a_picker_asks_for_a_query_only_once_it_is_worth_one() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let picker = hits
        .rect_of(&Target::Picker(Picks::Repo))
        .expect("the header has one");
    click(picker, &mut ui, &app, &hits);
    let shown = |ui: &Ui| {
        let (frame, _) = view(&app, ui, window(), &mut Fonts::embedded());
        frame
            .layers()
            .iter()
            .flat_map(|layer| layer.texts.iter())
            .any(|run| run.text.starts_with("worktree:"))
    };
    assert!(!shown(&ui), "two worktrees need no filter");
    if let Some(palette) = ui.palette_mut() {
        palette.query.push('a');
    }
    assert!(shown(&ui), "and it appears the moment something is typed");
}

#[test]
fn the_boundaries_a_run_left_come_back_where_they_stood() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    click(rail_edge(&hits), &mut ui, &app, &hits);
    drag(300.0, &mut ui, &app, &hits);
    release(&mut ui, &app, &hits);
    let kept = ui.split.panes();
    assert_eq!(Split::of(kept), ui.split);
}

#[test]
fn a_column_narrower_than_its_minimum_opens_at_it() {
    let panes = groove_types::Panes {
        rail: 1.0,
        agent: 1.0,
        sidebar: 1.0,
        commit: 1.0,
        band: 1.0,
        feed: 1.0,
    };
    let split = Split::of(panes);
    assert_eq!(split.rail, RAIL_MIN);
    assert_eq!(split.agent, AGENT_MIN);
    assert_eq!(split.band, crate::base::tokens::BAND_MIN);
    assert_eq!(split.feed, crate::base::tokens::FEED_MIN);
}
