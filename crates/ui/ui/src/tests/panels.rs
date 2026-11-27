//! The opaque panels: each headed by its band, ruled off under it; a folded one is its band alone.

use groove_gfx::{Color, Fonts, Frame, Rect};
use groove_ui_kit::base::style::Styles;
use groove_ui_kit::base::tokens::Tokens;

use crate::hit::Target;
use crate::layout::Layout;
use crate::tests::{full_app, window};
use crate::views::session::Mode;
use crate::{Ui, view};

/// Each quad of the frame in `color`, with its layer and its place in that layer.
fn quads(frame: &Frame, color: Color) -> Vec<(usize, usize, Rect)> {
    let layers = frame.layers().iter().enumerate();
    let all =
        layers.flat_map(|(at, one)| one.quads.iter().enumerate().map(move |(i, q)| (at, i, q)));
    all.filter(|(_, _, quad)| quad.color == color)
        .map(|(at, i, quad)| (at, i, quad.rect))
        .collect()
}

#[test]
fn every_open_panel_is_headed_by_its_band_ruled_off_under_it() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.manual = true;
    let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
    let layout = Layout::of(window(), &ui);
    let styles = Styles::new(app.config.theme(), Tokens::new(1.0));
    let bands = quads(&frame, styles.panel_head());
    let rules = quads(&frame, styles.line());
    let bar = Tokens::new(1.0).bar;
    for (name, panel) in [
        ("sessions", layout.sessions),
        ("feed", layout.feed),
        ("drawer", layout.manual),
    ] {
        let top = |rect: &Rect| {
            rect.y <= panel.y + 1.0
                && rect.x <= panel.x + 1.0
                && rect.right() >= panel.right() - 1.0
        };
        assert!(
            bands.iter().any(|(_, _, one)| top(one)),
            "{name}: its band at its top"
        );
        let under =
            |rect: &Rect| (rect.bottom() - (panel.y + bar)).abs() <= 1.0 && rect.w >= panel.w - 1.0;
        assert!(
            rules.iter().any(|(_, _, one)| under(one)),
            "{name}: ruled off under its band"
        );
    }
}

#[test]
fn a_set_of_tabs_stands_edge_to_edge_in_one_hairline_box() {
    let app = full_app();
    let mut ui = Ui::default();
    ui.session.mode = Mode::Diff;
    let (frame, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let at = |mode: Mode| hits.rect_of(&Target::ModeTab(mode)).expect("a tab");
    let (overview, diff, files) = (at(Mode::Overview), at(Mode::Diff), at(Mode::Files));
    let agent = at(Mode::Agent);
    assert!(
        (overview.right() - agent.x).abs() < 0.5 && (agent.right() - diff.x).abs() < 0.5,
        "no gap between them"
    );
    assert!((diff.right() - files.x).abs() < 0.5);
    let styles = Styles::new(app.config.theme(), Tokens::new(1.0));
    let quads = frame.layers().iter().flat_map(|one| one.quads.iter());
    let boxed = quads.clone().any(|quad| {
        quad.shape.stroke > 0.0
            && quad.color == styles.line()
            && (quad.rect.x - overview.x).abs() < 0.5
            && (quad.rect.right() - files.right()).abs() < 0.5
    });
    assert!(boxed, "one box around the set");
    let held = quads
        .into_iter()
        .find(|quad| quad.rect == diff && quad.color == styles.glass_chosen());
    assert_eq!(held.expect("the open one held").shape.radius, 0.0, "square");
    let texts = crate::tests::all_texts(&frame);
    assert!(
        texts.iter().any(|one| one == "DIFF"),
        "a word in capitals: {texts:?}"
    );
}

#[test]
fn the_defense_themes_lay_a_hex_lattice_on_the_deep_ground_and_the_others_none() {
    let lattice_in = |theme: groove_types::ThemeName| {
        let mut app = crate::tests::agent::sourced(full_app(), true, true);
        let mut config = app.config.config.clone().expect("a config");
        config.ui.theme = theme;
        app.config.config = Some(config);
        let ui = Ui::default();
        let (frame, _) = view(&app, &ui, window(), &mut Fonts::embedded());
        let color = Styles::new(theme, Tokens::new(1.0)).lattice();
        let unders = frame.layers().iter().flat_map(|one| one.under.iter());
        let laid: Vec<Rect> = unders
            .filter(|(_, c, shape)| Some(*c) == color && shape.lattice > 0.0)
            .map(|(rect, ..)| *rect)
            .collect();
        (laid, Layout::of(window(), &ui))
    };
    for theme in [
        groove_types::ThemeName::Defense,
        groove_types::ThemeName::DefenseLight,
    ] {
        let (laid, layout) = lattice_in(theme);
        assert!(
            laid.iter()
                .any(|one| one.intersect(layout.agent) == layout.agent),
            "{theme:?}: the lattice runs behind the agent's screen too"
        );
    }
    let (none, _) = lattice_in(groove_types::ThemeName::Mocha);
    assert!(none.is_empty(), "no other theme lays one");
}

#[test]
fn a_boundary_is_grabbed_on_the_border_the_user_sees() {
    use crate::layout::Edge;
    let app = full_app();
    let ui = Ui::default();
    let (_, hits) = view(&app, &ui, window(), &mut Fonts::embedded());
    let layout = Layout::of(window(), &ui);
    let at = |edge: Edge| hits.rect_of(&Target::Split(edge)).expect("its band");
    let across = |band: Rect, x: f32| band.x <= x && band.right() >= x;
    assert!(
        across(at(Edge::Rail), layout.sessions.right()),
        "the rail's border"
    );
    assert!(
        across(at(Edge::Aside), layout.sidebar.x),
        "the sidebar's border"
    );
    let feed = at(Edge::Feed);
    assert!(
        feed.y <= layout.feed.y && feed.bottom() >= layout.feed.y,
        "the feed's top"
    );
}

#[test]
fn signals_fire_across_the_lattice_only_while_the_session_s_agent_works() {
    use groove_types::{AgentStatus, SessionActivity, SessionId, ThemeName, Timestamp};
    let lit = |theme: ThemeName, status: AgentStatus| {
        let mut app = crate::tests::agent::sourced(full_app(), true, true);
        let mut config = app.config.config.clone().expect("a config");
        config.ui.theme = theme;
        app.config.config = Some(config);
        let id = app
            .session
            .selected
            .clone()
            .unwrap_or_else(|| SessionId::new("a"));
        let activity = SessionActivity {
            status,
            tool: None,
            asks: Vec::new(),
            changed_at: Timestamp::new(0),
            seen_at: None,
        };
        let agent = groove_controllers::agent_service::Agent {
            stream: None,
            permissions: Vec::new(),
            commands: Vec::new(),
            limits: Default::default(),
            windows: Vec::new(),
            chosen: None,
            activity,
            started_at: Timestamp::new(0),
            launch: 0,
        };
        app.agent.agents.push((id, agent));
        let ui = Ui::default();
        let mut metrics = window();
        metrics.tick = 1900;
        let (frame, _) = view(&app, &ui, metrics, &mut Fonts::embedded());
        let unders = frame.layers().iter().flat_map(|one| one.under.iter());
        let glows = unders.filter(|(_, _, shape)| shape.lattice > 0.0 && shape.blur > 0.0);
        (glows.count() > 0, crate::fires(&app, &ui))
    };
    assert_eq!(lit(ThemeName::Defense, AgentStatus::Working), (true, true));
    assert_eq!(
        lit(ThemeName::DefenseLight, AgentStatus::Working),
        (true, true)
    );
    assert_eq!(
        lit(ThemeName::Defense, AgentStatus::Idle),
        (false, false),
        "idle: still"
    );
    assert_eq!(
        lit(ThemeName::Mocha, AgentStatus::Working),
        (false, false),
        "no lattice"
    );
}
