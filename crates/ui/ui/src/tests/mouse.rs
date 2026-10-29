use groove_controllers::AppState;
use groove_gfx::Fonts;
use groove_types::{SessionId, WorktreeId};

use crate::hit::{Cursor, Hits, Target};
use crate::input::Key;
use crate::layout::{Edge, Layout, Split};
use crate::tests::{WINDOW, click, drag, full_app, metrics, press, release};
use crate::views::session::Tab;
use crate::{Ui, view};
use groove_ui_kit::base::tokens::{AGENT_MIN, RAIL_MIN, Tokens, WORKSPACE_MIN};

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
fn alt_w_picks_a_worktree_by_repo_and_branch_ranked() {
    let app = two_repos();
    let mut ui = Ui::default();
    let alt = crate::input::Modifiers {
        alt: true,
        ..Default::default()
    };
    press(Key::Char('w'), alt, &mut ui, &app);
    let rows = |ui: &Ui| {
        let palette = ui.palette().expect("the worktree palette");
        let rows = palette.options(&app).into_iter().map(|(label, _)| label);
        rows.collect::<Vec<_>>()
    };
    assert_eq!(
        rows(&ui),
        ["kiwi/explorer/alpha", "kiwi/fix/y", "pomme/main"]
    );
    crate::tests::typed("po/ma", &mut ui, &app);
    assert_eq!(rows(&ui).first().map(String::as_str), Some("pomme/main"));
    let picked = press(Key::Enter, Default::default(), &mut ui, &app);
    let select = groove_controllers::session::Command::SelectWorktree {
        session: SessionId::new("a"),
        worktree: WorktreeId::new("wt-2"),
    };
    assert_eq!(picked, [groove_controllers::Command::Session(select)]);
    assert!(ui.palette().is_none(), "the palette shuts");
}

/// The fixture's repo with a second worktree, and a second repo with one of its own.
fn two_repos() -> AppState {
    let mut app = full_app();
    let open = app.session.get_mut(&SessionId::new("a")).unwrap();
    let kiwi = open.repos[0].clone();
    let worktree = |id: &str, repo: &groove_types::Repo, branch: &str| groove_types::Worktree {
        id: WorktreeId::new(id),
        session: SessionId::new("a"),
        repo: repo.id.clone(),
        branch: branch.into(),
        path: format!("/code/worktrees/a/{}/{branch}", repo.project),
        base_ref: None,
        created_at: groove_types::Timestamp::new(0),
    };
    let pomme = groove_types::Repo {
        id: groove_types::RepoId::new("gitlab.example.com/g/pomme"),
        project: "pomme".into(),
        local_path: "/pool/pomme".into(),
        ..kiwi.clone()
    };
    open.add_worktree(kiwi.clone(), worktree("wt-3", &kiwi, "fix/y"));
    open.add_worktree(pomme.clone(), worktree("wt-2", &pomme, "main"));
    app
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
fn the_selected_tab_stands_on_a_rounded_ground_as_a_rail_row() {
    let app = full_app();
    let mut ui = Ui::default();
    let hits = regions(&app, &ui);
    let diff = hits.rect_of(&Target::Tab(Tab::Diff)).expect("the diff tab");
    click(diff, &mut ui, &app, &hits);
    let (frame, hits) = view(&app, &ui, metrics(1280, 800, 1.0), &mut Fonts::embedded());
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), Tokens::new(1.0));
    let held = |target: Target| {
        let at = hits.rect_of(&target).expect("a tab");
        let quads = frame.layers().iter().flat_map(|one| one.quads.iter());
        quads
            .into_iter()
            .any(|quad| quad.rect == at && quad.color == styles.glass_active())
    };
    assert!(held(Target::Tab(Tab::Diff)), "the selected tab");
    assert!(!held(Target::Tab(Tab::Overview)), "not the others");
}

#[test]
fn a_click_on_a_palette_row_runs_it() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char(';'), crate::tests::ALT_SHIFT, &mut ui, &app);
    let hits = regions(&app, &ui);
    let rect = hits
        .rect_of(&Target::PaletteRow(0))
        .expect("the palette lists its rows");
    click(rect, &mut ui, &app, &hits);
    assert_eq!(
        ui.surface,
        crate::Surface::Board,
        "the first row is Go › Board"
    );
    assert!(ui.palette().is_none());
}

#[test]
fn a_click_outside_the_palette_closes_it() {
    let app = full_app();
    let mut ui = Ui::default();
    press(Key::Char(';'), crate::tests::ALT_SHIFT, &mut ui, &app);
    let hits = regions(&app, &ui);
    let palette = hits.rect_of(&Target::Palette).expect("the palette");
    let rect = groove_gfx::Rect::new(palette.x - 40.0, palette.y + 40.0, 10.0, 10.0);
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
    let held = columns(ui.split);
    assert_eq!(held.board.w - held.agent.w, WORKSPACE_MIN);
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
    ui.session.tab = crate::views::session::Tab::Diff;
    let hits = regions(&app, &ui);
    let edge = hits
        .rect_of(&Target::Split(Edge::Sidebar))
        .expect("the diff tab has a sidebar to drag");
    let agent = wide(ui.split).agent.w;
    click(edge, &mut ui, &app, &hits);

    let width = WINDOW.0 as f32;
    drag(width - 340.0, &mut ui, &app, &hits);
    assert_eq!(
        wide(ui.split).sidebar.x,
        width - 340.0,
        "the edge under the pointer"
    );
    assert_eq!(
        wide(ui.split).agent.w,
        agent,
        "the workspace gives the room, not the agent"
    );

    drag(width - 600.0, &mut ui, &app, &hits);
    let sm = Tokens::new(1.0).sm;
    assert_eq!(
        wide(ui.split).workspace.w + sm * 2.0,
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
        feed: 1.0,
        manual: 1.0,
    };
    let split = Split::of(panes);
    assert_eq!(split.rail, RAIL_MIN);
    assert_eq!(split.agent, AGENT_MIN);
    assert_eq!(split.feed, groove_ui_kit::base::tokens::FEED_MIN);
}
