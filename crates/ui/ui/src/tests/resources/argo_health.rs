//! An Application's tab: its heading's revision, each resource's health from Argo CD.

use groove_controllers::Command;
use groove_controllers::cluster::Command as Cluster;
use groove_types::ResourceHealth;

const READER: &str = "resource";

use super::argo::failing;
use super::argo_sync::{managed, opened_application};
use super::fixture::runs;
use crate::tests::window;

fn app() -> (String, String, String) {
    let held = ["staging", "shop", "billing-acc"];
    (held[0].into(), held[1].into(), held[2].into())
}

/// The tab's stream held, and `read` landed on it.
fn treed(
    app_state: &mut groove_controllers::AppState,
    read: groove_controllers::cluster_service::Treed,
) {
    let trees = &mut app_state.cluster.store.trees;
    trees.lease(&app(), READER);
    trees.apply(&app(), read);
}

fn health(name: &str, status: &str) -> ResourceHealth {
    ResourceHealth {
        group: String::new(),
        kind: "Deployment".into(),
        namespace: Some("billing-acc".into()),
        name: name.into(),
        status: status.into(),
        reason: String::new(),
    }
}

#[test]
fn the_resources_view_asks_argo_cd_for_health_and_shows_it_by_resource() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.resources = vec![
        managed("Deployment", "billing-web", "Synced", 0),
        managed("Deployment", "billing-worker", "Synced", 0),
    ];
    let (mut app_state, mut ui) = opened_application(object);
    ui.session.resources.tab_mut().expect("its tab").argo.filter =
        Some(crate::views::session::resources::Filter::All);
    let asked = crate::frame_commands(&app_state, &ui, window());
    let follow = Command::Cluster(Cluster::FollowArgo {
        reader: READER.into(),
        app: app(),
    });
    assert!(asked.contains(&follow), "{asked:?}");

    let read = vec![
        health("billing-web", "Degraded"),
        health("billing-worker", "Healthy"),
    ];
    treed(&mut app_state, Ok(read));
    let texts: Vec<String> = runs(&app_state, &ui)
        .into_iter()
        .map(|one| one.text)
        .collect();
    for shown in ["HEALTH", "Degraded", "Healthy", "1 degraded"] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    assert!(!texts.iter().any(|one| one.starts_with("no health")));
    let asked = crate::frame_commands(&app_state, &ui, window());
    assert!(!asked.contains(&follow), "a stream held is not asked again");
}

#[test]
fn the_view_opens_on_what_is_degraded_out_of_sync_or_not() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.operation = None;
    part.resources = vec![
        managed("Deployment", "billing-web", "OutOfSync", 0),
        managed("Deployment", "billing-api", "OutOfSync", 0),
    ];
    let (mut app_state, ui) = opened_application(object);
    let read = vec![
        health("billing-web", "Degraded"),
        health("billing-api", "Healthy"),
    ];
    treed(&mut app_state, Ok(read));
    let texts: Vec<String> = runs(&app_state, &ui)
        .into_iter()
        .map(|one| one.text)
        .collect();
    for shown in ["degraded · 1", "out of sync · 2", "billing-web"] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    assert!(!texts.iter().any(|one| one == "billing-api"), "{texts:?}");
}

#[test]
fn a_refused_read_says_what_argocd_answered() {
    let (mut app_state, ui) = opened_application(failing());
    let why = "rpc error: code = Unauthenticated desc = invalid session\nmore".to_string();
    treed(&mut app_state, Err(why));
    let texts: Vec<String> = runs(&app_state, &ui)
        .into_iter()
        .map(|one| one.text)
        .collect();
    let said = "no health: rpc error: code = Unauthenticated desc = invalid session";
    assert!(texts.iter().any(|one| one == said), "{texts:?}");
}

#[test]
fn the_heading_shows_the_commit_its_first_git_source_synced_to() {
    let sha = "e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0";
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.sources[0].synced = Some(sha.into());
    let (app_state, ui) = opened_application(object);
    let texts: Vec<String> = runs(&app_state, &ui)
        .into_iter()
        .map(|one| one.text)
        .collect();
    assert!(texts.iter().any(|one| one == "e9f8a7b"), "{texts:?}");
}
