//! A maintenance task's alerts: where the window stands, folded or open, and what it lists.

use groove_types::{Alert, SessionId, Timestamp};

use groove_gfx::Rect;

use crate::Ui;
use crate::hit::Target;
use crate::input::{Input, handle};
use crate::layout::Layout;
use crate::tests::{app, metrics, render};

fn firing(key: &str, priority: &str) -> Alert {
    Alert {
        id: key.into(),
        title: format!("{key} is firing"),
        description: String::new(),
        dedup_key: key.into(),
        source_url: None,
        priority: Some(priority.into()),
        cluster: Some("prod-eu".into()),
        namespace: Some("velero".into()),
        environment: Some("Production".into()),
        created_at: Timestamp::new(0),
    }
}

/// The fixture with its selected session marked as maintenance, settled.
fn maintaining() -> (groove_controllers::AppState, Ui) {
    let mut app = app();
    let id = SessionId::new("a");
    app.session.selected = Some(id.clone());
    app.session
        .get_mut(&id)
        .expect("the fixture's session")
        .state
        .maintenance = true;
    app.alert.firing = vec![firing("velero", "Critical"), firing("disk", "Warning")];
    let mut ui = Ui::default();
    ui.split.agent = 1000.0;
    ui.settle(&app);
    (app, ui)
}

#[test]
fn open_the_alerts_stand_over_the_agent_which_keeps_the_rest_of_its_height() {
    let (_, ui) = maintaining();
    let window = metrics(1280, 800, 1.0);
    let mut bare_ui = Ui::default();
    bare_ui.split.agent = 1000.0;
    let bare = Layout::of(window, &bare_ui);
    let held = Layout::of(window, &ui);
    assert!(held.alerts.h > window.tokens().bar, "{:?}", held.alerts);
    assert_eq!(
        held.alerts.bottom(),
        groove_ui_kit::base::tokens::ALERTS_TALL
    );
    assert_eq!((held.alerts.x, held.alerts.w), (bare.agent.x, bare.agent.w));
    assert!(held.agent.y > held.alerts.bottom(), "the agent under them");
    assert_eq!(held.agent.bottom(), bare.agent.bottom());
}

#[test]
fn folded_the_alerts_are_their_header_in_the_bar_over_the_agent() {
    let (_, mut ui) = maintaining();
    ui.alerts.open = false;
    let window = metrics(1280, 800, 1.0);
    let held = Layout::of(window, &ui);
    assert_eq!(held.alerts.h, window.tokens().bar);
    assert!(
        held.alerts.bottom() <= held.agent.y,
        "over the agent's screen"
    );
    let mut bare = Ui::default();
    bare.split.agent = 1000.0;
    assert_eq!(held.agent, Layout::of(window, &bare).agent);
}

#[test]
fn with_everything_folded_the_alerts_take_the_bar_between_the_sessions_header_and_the_strip() {
    let (_, mut ui) = maintaining();
    (ui.rail.stowed, ui.rail.folded, ui.session.stowed) = (true, true, true);
    let held = Layout::of(metrics(1280, 800, 1.0), &ui);
    assert!(
        held.alerts.x > held.sessions.right(),
        "after the sessions header"
    );
    assert!(
        held.alerts.right() < held.strip.x,
        "before the strip's capsule"
    );
    assert_eq!(held.alerts.y, held.sessions.y, "on the same bar");
}

#[test]
fn the_alerts_count_by_rank_and_list_on_one_line_of_a_grid_with_a_button_to_investigate() {
    let (app, ui) = maintaining();
    let (texts, _) = render(&app, &ui, metrics(1600, 900, 1.0));
    for said in [
        "1 critical",
        "1 warning",
        "PRIORITY",
        "ENVIRONMENT",
        "CLUSTER",
        "NAMESPACE",
        "Critical",
        "velero is firing",
        "Production",
        "prod-eu",
        "Investigate",
    ] {
        assert!(texts.iter().any(|one| one == said), "{said}: {texts:?}");
    }
}

/// One press, a move and a release, from `at`.
fn dragged(ui: &mut Ui, app: &groove_controllers::AppState, hits: &crate::hit::Hits, at: Rect) {
    let (x, y) = (at.x + 1.0, at.y + 1.0);
    let window = metrics(1600, 900, 1.0);
    let press = Input::Press {
        x,
        y,
        mods: Default::default(),
    };
    handle(press, ui, app, hits, window);
    handle(Input::Move { x: x + 40.0, y }, ui, app, hits, window);
    handle(Input::Release, ui, app, hits, window);
}

#[test]
fn an_alerts_column_drags_its_width_and_its_header_orders_the_rows() {
    let (app, mut ui) = maintaining();
    let (_, hits) = render(&app, &ui, metrics(1600, 900, 1.0));
    let header = |hits: &crate::hit::Hits, label: &str| {
        hits.rect_of(&Target::AlertColumn(label.into()))
            .expect(label)
    };
    let before = header(&hits, "priority").w;
    let edge = hits
        .rect_of(&Target::AlertEdge("priority".into()))
        .expect("the edge");
    dragged(&mut ui, &app, &hits, edge);
    let wide = ui
        .alerts
        .widths
        .get("priority")
        .copied()
        .unwrap_or_default();
    assert!(
        (wide - (before + 40.0)).abs() < 0.01,
        "{wide} after {before}"
    );

    let (_, hits) = render(&app, &ui, metrics(1600, 900, 1.0));
    dragged(&mut ui, &app, &hits, header(&hits, "alert"));
    assert_eq!(ui.alerts.sort, Some(("alert".to_string(), false)));
    let (texts, _) = render(&app, &ui, metrics(1600, 900, 1.0));
    let at = |said: &str| texts.iter().position(|one| one == said).expect(said);
    assert!(
        at("disk is firing") < at("velero is firing"),
        "by the alert's name"
    );
}

#[test]
fn the_alerts_foot_is_dragged_and_the_agent_follows() {
    let (_, mut ui) = maintaining();
    let window = metrics(1280, 800, 1.0);
    let before = Layout::of(window, &ui);
    ui.split
        .drag(crate::layout::Edge::Alerts, 420.0, (1280.0, 800.0), false);
    let after = Layout::of(window, &ui);
    assert_eq!(after.alerts.bottom(), 420.0);
    assert!(after.alerts.h > before.alerts.h);
    assert!(after.agent.y > after.alerts.bottom());
    assert_eq!(after.agent.bottom(), before.agent.bottom());
}

#[test]
fn a_session_not_marked_has_no_alerts() {
    let mut app = app();
    app.alert.firing = vec![firing("velero", "Critical")];
    let mut ui = Ui::default();
    ui.settle(&app);
    assert!(Layout::of(metrics(1280, 800, 1.0), &ui).alerts.is_empty());
}

/// One press and release on `at`, and what it asked for.
fn clicked(
    ui: &mut Ui,
    app: &groove_controllers::AppState,
    hits: &crate::hit::Hits,
    at: Rect,
) -> Vec<groove_controllers::Command> {
    let (x, y) = (at.x + 1.0, at.y + 1.0);
    let window = metrics(1600, 900, 1.0);
    let press = Input::Press {
        x,
        y,
        mods: Default::default(),
    };
    let mut asked = handle(press, ui, app, hits, window);
    asked.extend(handle(Input::Release, ui, app, hits, window));
    asked
}

#[test]
fn a_row_takes_the_keys_and_only_its_button_or_enter_investigates() {
    let (app, mut ui) = maintaining();
    let (_, hits) = render(&app, &ui, metrics(1600, 900, 1.0));
    let row = hits
        .rect_of(&Target::AlertRow("disk".into()))
        .expect("the row");
    assert!(
        clicked(&mut ui, &app, &hits, row).is_empty(),
        "a row investigates nothing"
    );
    assert_eq!(ui.focus, crate::Focus::Alerts);
    assert_eq!(ui.alerts.selected.as_deref(), Some("disk"));

    let investigate =
        groove_controllers::Command::Alert(groove_controllers::alert::Command::Investigate {
            parent: SessionId::new("a"),
            dedup_key: "disk".into(),
        });
    let enter = crate::tests::press(crate::input::Key::Enter, Default::default(), &mut ui, &app);
    assert_eq!(enter, std::slice::from_ref(&investigate));
    let up = crate::tests::press(crate::input::Key::Up, Default::default(), &mut ui, &app);
    assert!(up.is_empty());
    assert_eq!(
        ui.alerts.selected.as_deref(),
        Some("velero"),
        "the arrows step the rows"
    );
    let button = hits
        .rect_of(&Target::Investigate("disk".into()))
        .expect("the button");
    assert_eq!(clicked(&mut ui, &app, &hits, button), [investigate]);
}

#[test]
fn alt_i_unfolds_the_alerts_gives_them_the_keys_then_folds_them() {
    let (app, mut ui) = maintaining();
    ui.alerts.open = false;
    let mods = crate::input::Modifiers {
        alt: true,
        ..Default::default()
    };
    crate::tests::press(crate::input::Key::Char('i'), mods, &mut ui, &app);
    assert!(ui.alerts.open, "unfolded");
    assert_eq!(ui.focus, crate::Focus::Alerts);
    crate::tests::press(crate::input::Key::Char('i'), mods, &mut ui, &app);
    assert!(!ui.alerts.open, "folded away");
    assert_eq!(ui.focus, crate::Focus::Agent);
}
