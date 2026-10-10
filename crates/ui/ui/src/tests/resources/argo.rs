//! Argo CD's Applications: their own list, and an Application's tab on its last sync.

use groove_controllers::cluster_service::Batch;
use groove_controllers::{AppState, cluster_service::Followed};
use groove_types::{
    AppPart, AppSource, Described, Destination, KubeKind, ObjectRow, Operation, SyncedResource,
    TableColumn, WatchKey,
};

use super::tab::runs;
use super::{kind, listing};
use crate::Ui;
use crate::hit::Target;
use crate::tests::{click, window};
use crate::views::session::resources::Link;

fn applications() -> KubeKind {
    KubeKind {
        group: "argoproj.io".into(),
        version: "v1alpha1".into(),
        ..kind("argoproj.io", "Application", "applications", true)
    }
}

/// The session's list on Applications, one of them failing.
fn listed() -> (AppState, Ui) {
    let (mut app, mut ui) = listing();
    let mut kinds = app.cluster.store.kinds("staging").expect("kinds").to_vec();
    kinds.push(applications());
    app.cluster.store.set_kinds("staging", kinds);
    let key = WatchKey {
        context: "staging".into(),
        kind: applications(),
        namespace: Some("paxone".into()),
        selector: None,
    };
    let names = [
        "Name",
        "Project",
        "Sync Status",
        "Health Status",
        "Auto-sync",
        "Revision",
    ];
    let columns = names
        .map(|name| TableColumn {
            name: name.into(),
            priority: 0,
            date: false,
        })
        .to_vec();
    let row = |name: &str, sync: &str, health: &str| ObjectRow {
        uid: format!("paxone/{name}"),
        name: name.into(),
        namespace: Some("paxone".into()),
        version: "1".into(),
        cells: [
            name,
            "data-science",
            sync,
            health,
            "auto · prune · heal",
            "a1b2c3d",
        ]
        .map(String::from)
        .to_vec(),
        aging: Vec::new(),
    };
    let rows = vec![
        row("pythie-acc", "OutOfSync", "Degraded"),
        row("pythie-prod", "Synced", "Healthy"),
    ];
    app.cluster.store.lease(&key, "resources");
    app.cluster
        .store
        .apply(&key, Batch::Reset { columns, rows });
    ui.session.resources.kind = Some(applications());
    (app, ui)
}

fn failing() -> Described {
    let operation = Operation {
        phase: "Failed".into(),
        message: "one or more objects failed to apply".into(),
        revision: Some("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678".into()),
        started: None,
        finished: None,
        retries: 2,
        by: "auto-sync".into(),
        failed: vec![SyncedResource {
            kind: "Job".into(),
            name: "pythie-migrate".into(),
            namespace: Some("paxone".into()),
            status: "Synced".into(),
            message: "Job has reached the specified backoff limit".into(),
            hook: true,
        }],
    };
    let part = AppPart {
        project: "data-science".into(),
        sync: "OutOfSync".into(),
        health: "Degraded".into(),
        sources: vec![AppSource {
            repo: "https://gitlab.wiremind.io/pythie-cayzn-deploy.git".into(),
            path: Some("envs/acc".into()),
            chart: None,
            target: "main".into(),
            values: Vec::new(),
            reference: None,
            synced: Some("e9f8a7b".into()),
        }],
        synced: Some("e9f8a7b".into()),
        destination: Destination {
            server: Some("https://10.0.0.1:6443".into()),
            name: None,
            namespace: Some("pythie-acc".into()),
        },
        automated: Some((true, true)),
        operation: Some(operation),
        conditions: Vec::new(),
    };
    Described {
        uid: "paxone/pythie-acc".into(),
        name: "pythie-acc".into(),
        namespace: Some("paxone".into()),
        labels: Vec::new(),
        annotations: Vec::new(),
        created: None,
        owners: Vec::new(),
        replicas: None,
        selector: Vec::new(),
        pod: None,
        app: Some(Box::new(part)),
        event: None,
        yaml: "".into(),
    }
}

#[test]
fn applications_list_their_own_columns_and_one_opens_on_its_failed_sync() {
    let (mut app, mut ui) = listed();
    let texts: Vec<String> = runs(&app, &ui).into_iter().map(|one| one.text).collect();
    for shown in [
        "PROJECT",
        "SYNC STATUS",
        "HEALTH STATUS",
        "AUTO-SYNC",
        "pythie-acc",
        "auto · prune · heal",
    ] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), window().tokens());
    let colour = |text: &str| {
        runs(&app, &ui)
            .into_iter()
            .find(|one| one.text == text)
            .map(|one| one.style.color)
    };
    assert_eq!(
        colour("Degraded"),
        Some(styles.color(groove_ui_kit::base::style::Role::Bad))
    );
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
    let (_, hits) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let row = hits
        .rect_of(&Target::ResourceRow("paxone/pythie-acc".into()))
        .expect("its row");
    click(row, &mut ui, &app, &hits);
    let link = Link {
        context: "staging".into(),
        kind: applications(),
        namespace: Some("paxone".into()),
        name: "pythie-acc".into(),
    };
    app.cluster.store.follows.lease(&link.key(), "resource");
    app.cluster
        .store
        .follows
        .apply(&link.key(), Followed::Reset(vec![failing()]));
    let drawn = runs(&app, &ui);
    let texts: Vec<&str> = drawn.iter().map(|one| one.text.as_str()).collect();
    for shown in [
        "OutOfSync",
        "Degraded",
        "OPERATION",
        "✗ Sync failed",
        "pythie-migrate",
        "auto · prune · heal",
        "SOURCES",
        "pythie-cayzn-deploy · envs/acc",
        "gitlab.wiremind.io/pythie-cayzn-deploy",
        "e9f8a7b",
    ] {
        assert!(texts.contains(&shown), "{shown}: {texts:?}");
    }
    assert!(
        !texts.contains(&"RELATIONS"),
        "an Application stands among no owners"
    );
    let events = crate::views::session::resources::Section::Events;
    let tab = ui.session.resources.tab().expect("its tab");
    assert!(tab.shut.contains(&events), "events fold until opened");
    let (frame, _) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let red = styles.tint(groove_ui_kit::base::style::Role::Bad);
    let quads = frame.layers().iter().flat_map(|one| one.quads.iter());
    assert!(
        quads.into_iter().any(|one| one.color == red),
        "the operation on its red ground"
    );
}
