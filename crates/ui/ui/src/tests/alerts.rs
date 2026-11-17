//! A maintenance task's alerts: where the window stands, folded or open, and what it lists.

use groove_types::{Alert, SessionId, Timestamp};

use crate::Ui;
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
    ui.settle(&app);
    (app, ui)
}

#[test]
fn open_the_alerts_stand_over_the_agent_which_keeps_the_rest_of_its_height() {
    let (_, ui) = maintaining();
    let window = metrics(1280, 800, 1.0);
    let bare = Layout::of(window, &Ui::default());
    let held = Layout::of(window, &ui);
    assert!(held.alerts.h > window.tokens().bar, "{:?}", held.alerts);
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
    assert_eq!(held.agent, Layout::of(window, &Ui::default()).agent);
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
fn the_alerts_count_by_rank_and_offer_to_investigate() {
    let (app, ui) = maintaining();
    let (texts, _) = render(&app, &ui, metrics(1280, 800, 1.0));
    for said in [
        "1 critical",
        "1 warning",
        "velero is firing",
        "prod-eu/velero",
        "investigate",
    ] {
        assert!(texts.iter().any(|one| one == said), "{said}: {texts:?}");
    }
}

#[test]
fn a_session_not_marked_has_no_alerts() {
    let mut app = app();
    app.alert.firing = vec![firing("velero", "Critical")];
    let mut ui = Ui::default();
    ui.settle(&app);
    assert!(Layout::of(metrics(1280, 800, 1.0), &ui).alerts.is_empty());
}
