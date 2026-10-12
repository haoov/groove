//! The Resources tab across sessions.

use groove_controllers::Command;
use groove_controllers::session::Command as Session;
use groove_types::{SessionId, Timestamp};

use super::fixture::{crashing, lands, link, opened};
use super::{clustered, key, on, pods};
use crate::hit::Target;
use crate::tests::click;
use crate::tests::settings::drawn;
use crate::views::session::Tab;

#[test]
fn a_session_holding_no_cluster_shows_its_overview_not_the_last_object() {
    let (mut app, mut ui) = opened();
    lands(&mut app, &link("api-0", pods()).key(), vec![crashing()]);
    let (texts, hits) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "CONDITIONS"), "{texts:?}");
    let b = SessionId::new("b");
    let row = hits.rect_of(&Target::Session(b.clone())).expect("b's row");
    let asked = click(row, &mut ui, &app, &hits);
    let select = Command::Session(Session::Open { session: b.clone() });
    assert!(asked.contains(&select), "{asked:?}");
    app.session.select(&b, Timestamp::new(0));
    ui.settle(&app);
    assert_eq!(ui.session.tab, Tab::Overview);
    let (texts, _) = drawn(&app, &ui);
    assert!(!texts.iter().any(|one| one == "CONDITIONS"), "{texts:?}");
}

#[test]
fn each_session_keeps_its_own_object_tabs() {
    let (mut app, mut ui) = opened();
    ui.settle(&app);
    assert_eq!(ui.session.resources.opened.len(), 1, "a's tab");
    let (a, b) = (SessionId::new("a"), SessionId::new("b"));
    app.session.select(&b, Timestamp::new(0));
    ui.settle(&app);
    assert!(
        ui.session.resources.opened.is_empty(),
        "b shows none of a's tabs"
    );
    app.session.select(&a, Timestamp::new(0));
    ui.settle(&app);
    assert_eq!(ui.session.resources.opened.len(), 1, "a's tab comes back");
}

#[test]
fn a_session_left_while_searching_comes_back_with_no_search_holding_the_keys() {
    let (mut app, mut ui) = super::listing();
    ui.settle(&app);
    (ui.session.resources.typing, ui.session.resources.filtering) = (true, true);
    let (a, b) = (SessionId::new("a"), SessionId::new("b"));
    app.session.select(&b, Timestamp::new(0));
    ui.settle(&app);
    app.session.select(&a, Timestamp::new(0));
    ui.settle(&app);
    let held = &ui.session.resources;
    assert!(
        !held.typing && !held.filtering,
        "the searches let go of the keys"
    );
}

#[test]
fn a_session_closed_and_opened_again_starts_its_kubernetes_tab_clean() {
    let (mut app, mut ui) = opened();
    ui.settle(&app);
    let (a, b) = (SessionId::new("a"), SessionId::new("b"));
    app.session.select(&b, Timestamp::new(0));
    ui.settle(&app);
    let closed = app.session.close(&a).expect("a was open");
    ui.settle(&app);
    app.session.open.push(closed);
    app.session.select(&a, Timestamp::new(0));
    ui.settle(&app);
    assert!(
        ui.session.resources.opened.is_empty(),
        "the tabs of the session closed went with it"
    );
}

/// Explorer `a` made the task's session `task`, as the controller does.
fn promote(app: &mut groove_controllers::AppState) -> SessionId {
    let a = SessionId::new("a");
    let mut session = app.session.get(&a).expect("a is open").session.clone();
    session.id = SessionId::new("task");
    let worktrees = app.session.get(&a).expect("a").worktrees.clone();
    app.session.promoted(&a, session, worktrees);
    SessionId::new("task")
}

#[test]
fn an_explorer_made_a_task_s_session_keeps_its_kubernetes_tabs() {
    let (mut app, mut ui) = opened();
    ui.settle(&app);
    promote(&mut app);
    ui.settle(&app);
    assert_eq!(ui.session.resources.opened.len(), 1, "shown, it keeps them");

    let (mut app, mut ui) = opened();
    ui.settle(&app);
    app.session.select(&SessionId::new("b"), Timestamp::new(0));
    ui.settle(&app);
    let task = promote(&mut app);
    ui.settle(&app);
    app.session.select(&task, Timestamp::new(0));
    ui.settle(&app);
    assert_eq!(ui.session.resources.opened.len(), 1, "kept aside, too");
}

#[test]
fn leaving_a_session_lets_its_object_go_and_coming_back_follows_it_again() {
    use groove_controllers::cluster::Command as Cluster;
    let (mut app, mut ui) = opened();
    let follow = link("api-0", pods()).key();
    app.cluster.store.follows.lease(&follow, "resource");
    ui.settle(&app);
    let b = SessionId::new("b");
    let open = app.session.get_mut(&b).expect("b");
    open.clusters = vec![on("staging", Some("other"))];
    app.session.select(&b, Timestamp::new(0));
    ui.settle(&app);
    let asked = clustered(&app, &ui);
    let release = Cluster::Release {
        reader: "resource".into(),
    };
    assert!(asked.contains(&Command::Cluster(release)), "{asked:?}");
    let watch = |one: &Command| {
        matches!(one, Command::Cluster(Cluster::Watch { key: at, .. })
            if *at == key("staging", Some("other")))
    };
    assert!(asked.iter().any(watch), "b's list watches: {asked:?}");
    app.cluster.store.follows.release("resource");

    app.session.select(&SessionId::new("a"), Timestamp::new(0));
    ui.settle(&app);
    let asked = clustered(&app, &ui);
    let again = Cluster::Follow {
        reader: "resource".into(),
        key: Box::new(follow),
    };
    assert!(asked.contains(&Command::Cluster(again)), "{asked:?}");
}
