//! An object's own tab: opened from its row, what it reads, and what it draws.

use groove_controllers::cluster::Command as Cluster;
use groove_controllers::{AppState, Command};
use groove_types::{Described, EventRow, FollowKey, Timestamp, Usage};

use super::fixture::{base, crashing, lands, link, opened, runs, unfold, unfold_events, wheel};
use super::{kind, pods};
use crate::Ui;
use crate::hit::Target;
use crate::tests::settings::drawn;
use crate::tests::{click, window};

/// The texts drawn on a wide screen, where the tab's columns have their room.
fn wide(app: &AppState, ui: &Ui) -> Vec<String> {
    let metrics = crate::tests::metrics(1920, 1440, 1.0);
    let (frame, _) = crate::view(app, ui, metrics, &mut groove_gfx::Fonts::embedded());
    let texts = frame.layers().iter().flat_map(|layer| layer.texts.iter());
    let small = groove_ui_kit::base::style::Styles::new(app.config.theme(), metrics.tokens())
        .small(groove_ui_kit::base::style::Role::Text)
        .size;
    for shown in ["gra9-node-2", "Ready"] {
        let one = texts.clone().find(|one| one.text == shown);
        assert!(
            one.is_none_or(|one| one.style.size == small),
            "{shown} at the small size"
        );
    }
    texts.map(|one| one.text.clone()).collect()
}

#[test]
fn a_row_opens_its_object_s_tab_which_follows_it_by_name() {
    let (app, ui) = opened();
    let held = &ui.session.resources;
    assert_eq!((held.opened.len(), held.showing), (1, Some(0)));
    let (texts, _) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "reading api-0…"), "{texts:?}");
    let asked = crate::frame_commands(&app, &ui, window());
    let follow = Cluster::Follow {
        reader: "resource".into(),
        key: Box::new(link("api-0", pods()).key()),
    };
    assert!(asked.contains(&Command::Cluster(follow)), "{asked:?}");
}

/// What the described view of the crashing pod shows.
const DESCRIBED: [&str; 20] = [
    "CrashLoopBackOff",
    "waiting · CrashLoopBackOff",
    "worker",
    "14",
    "gra9-node-2",
    "CONDITIONS",
    "SUMMARY",
    "RELATIONS · 2",
    "owned by",
    "worker-5d6b",
    "2/3 ready",
    "uses",
    "paxone-db-legacy",
    "env worker",
    "BackOff",
    "×187",
    "180m / 1",
    "700Mi / 1Gi",
    "http 8080/TCP",
    "tls → /etc/tls ro",
];

#[test]
fn a_pod_read_asks_its_events_owners_services_usage_and_release_and_draws_them() {
    let (mut app, mut ui) = opened();
    lands(&mut app, &link("api-0", pods()).key(), vec![crashing()]);
    let asked = crate::frame_commands(&app, &ui, window());
    let follows: Vec<FollowKey> = asked
        .iter()
        .filter_map(|one| match one {
            Command::Cluster(Cluster::Follow { key, .. }) => Some((**key).clone()),
            _ => None,
        })
        .collect();
    let fields: Vec<Option<&str>> = follows.iter().map(|one| one.fields.as_deref()).collect();
    assert!(
        fields.contains(&Some("involvedObject.uid=paxone/api-0")),
        "{fields:?}"
    );
    assert!(
        fields.contains(&Some("metadata.name=worker-5d6b")),
        "the owner: {fields:?}"
    );
    assert!(
        follows.iter().any(|one| one.kind.kind == "Service"),
        "the namespace's services"
    );
    let pod = (
        "staging".to_string(),
        "paxone".to_string(),
        "api-0".to_string(),
    );
    assert!(asked.contains(&Command::Cluster(Cluster::Usage { pod: pod.clone() })));
    let release = (
        "staging".to_string(),
        "paxone".to_string(),
        "paxone-staging".to_string(),
    );
    assert!(asked.contains(&Command::Cluster(Cluster::HelmRevision { release })));

    let events = FollowKey {
        fields: Some("involvedObject.uid=paxone/api-0".into()),
        ..follows[1].clone()
    };
    let backoff = EventRow {
        warning: true,
        reason: "BackOff".into(),
        count: 187,
        last: None,
        message: "Back-off restarting".into(),
        about: "pod/api-0".into(),
    };
    lands(
        &mut app,
        &events,
        vec![Described {
            event: Some(backoff),
            ..base("e1", "e1")
        }],
    );
    let deploy = link(
        "worker-5d6b",
        kind("apps", "ReplicaSet", "replicasets", true),
    );
    lands(
        &mut app,
        &deploy.key(),
        vec![Described {
            replicas: Some((2, 3)),
            ..base("rs", "worker-5d6b")
        }],
    );
    app.cluster.store.follows.set_usage(
        pod,
        Some(Usage {
            containers: vec![("worker".into(), 0.18, 734_003_200.0)],
        }),
        Timestamp::new(0),
    );
    unfold_events(&app, &mut ui);
    unfold(
        &app,
        &mut ui,
        crate::views::session::resources::Section::Relations,
    );
    let texts = wide(&app, &ui);
    for shown in DESCRIBED {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
}

#[test]
fn a_container_not_ready_stands_unfolded_and_its_usage_tells_request_and_limit_under_the_pointer() {
    let (mut app, mut ui) = opened();
    let key = link("api-0", pods()).key();
    lands(&mut app, &key, vec![crashing()]);
    let shown = |app: &AppState, ui: &Ui| -> Vec<String> {
        runs(app, ui).into_iter().map(|one| one.text).collect()
    };
    let texts = shown(&app, &ui);
    for one in ["NAME", "MEMORY", "ports", "http 8080/TCP", "mounts"] {
        assert!(texts.iter().any(|text| text == one), "{one}: {texts:?}");
    }
    assert!(!texts.iter().any(|text| text == "512Mi"), "no tip at rest");
    ui.hover = Some(Target::ContainerUsage("worker".into(), true));
    let texts = shown(&app, &ui);
    for one in ["used", "request", "512Mi", "limit", "1Gi"] {
        assert!(texts.iter().any(|text| text == one), "{one}: {texts:?}");
    }
    ui.hover = None;
    let metrics = crate::tests::metrics(1920, 1440, 1.0);
    let (_, hits) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let row = hits.rect_of(&Target::ResourceFold("container/worker".into(), false));
    click(row.expect("the worker's row"), &mut ui, &app, &hits);
    assert!(
        !shown(&app, &ui).iter().any(|text| text == "http 8080/TCP"),
        "a click folds it"
    );
}

#[test]
fn the_wheel_scrolls_the_one_described_column() {
    let (mut app, mut ui) = opened();
    let key = link("api-0", pods()).key();
    let mut tall = crashing();
    tall.labels = (0..40)
        .map(|at| (format!("label-{at:02}"), "x".into()))
        .collect();
    lands(&mut app, &key, vec![tall]);
    let events = FollowKey {
        fields: Some("involvedObject.uid=paxone/api-0".into()),
        kind: kind("", "Event", "events", true),
        ..key.clone()
    };
    let event = |at: usize| Described {
        event: Some(EventRow {
            warning: false,
            reason: format!("Reason{at:02}"),
            count: 1,
            last: None,
            message: "m".into(),
            about: "pod/api-0".into(),
        }),
        ..base(&format!("e{at}"), &format!("e{at}"))
    };
    lands(&mut app, &events, (0..40).map(event).collect());
    let shown = ui.session.resources.tab_mut().expect("the tab");
    shown
        .shut
        .remove(&crate::views::session::resources::Section::Events);
    let at = |ui: &Ui, text: &str| {
        runs(&app, ui)
            .iter()
            .find(|one| one.text == text)
            .map(|one| one.y)
    };
    let (label, reason) = (at(&ui, "label-00"), at(&ui, "Reason39"));
    let metrics = crate::tests::metrics(1920, 1440, 1.0);
    let (_, hits) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let middle = hits
        .rect_of(&Target::ResourceView(
            crate::views::session::resources::View::Describe,
        ))
        .map(|one| (one.x - 400.0, one.bottom() + 200.0))
        .expect("the panel");
    wheel(&app, &mut ui, middle, (0.0, -2400.0));
    assert!(
        ui.session
            .resources
            .tab()
            .is_some_and(|one| one.scroll > 0.0),
        "the wheel moved the panel"
    );
    assert!(at(&ui, "label-00") < label, "the labels went up with it");
    let now = at(&ui, "Reason39");
    assert!(
        now.is_some() && (reason.is_none() || now < reason),
        "the events went up or came into view"
    );
}

#[test]
fn a_section_s_heading_folds_it_and_the_annotations_start_folded() {
    use crate::views::session::resources::Section;
    let (mut app, mut ui) = opened();
    lands(&mut app, &link("api-0", pods()).key(), vec![crashing()]);
    let texts = |ui: &Ui| {
        runs(&app, ui)
            .into_iter()
            .map(|one| one.text)
            .collect::<Vec<_>>()
    };
    assert!(
        !texts(&ui)
            .iter()
            .any(|one| one == "meta.helm.sh/release-name"),
        "annotations folded"
    );
    let metrics = crate::tests::metrics(1920, 1440, 1.0);
    let (_, hits) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let annotations = hits
        .rect_of(&Target::ResourceSection(Section::Annotations))
        .expect("its heading");
    click(annotations, &mut ui, &app, &hits);
    assert!(
        texts(&ui)
            .iter()
            .any(|one| one == "meta.helm.sh/release-name"),
        "annotations open"
    );
    let (_, hits) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let summary = hits
        .rect_of(&Target::ResourceSection(Section::Summary))
        .expect("its heading");
    click(summary, &mut ui, &app, &hits);
    assert!(
        !texts(&ui).iter().any(|one| one == "pod ip"),
        "the summary folded"
    );
}

#[test]
fn behind_an_object_s_tab_the_list_lets_its_watchers_go_and_its_ages_rest() {
    use groove_controllers::cluster_service::{Batch, Delta};
    use groove_types::{Aging, ObjectRow};
    let (mut app, ui) = opened();
    let young = ObjectRow {
        aging: vec![Aging {
            cell: 1,
            since: Timestamp::new(1_000),
            said: 0,
            lead: None,
            turn: Timestamp::default(),
        }],
        ..super::row("young", "paxone", "Running")
    };
    let listed = super::key("staging", Some("paxone"));
    let put = Batch::Changes(vec![Delta::Put(young)]);
    app.cluster.store.apply(&listed, put);
    let now = Timestamp::new(1_179);
    let at = crate::Metrics { now, ..window() };
    let asked = crate::frame_commands(&app, &ui, at);
    let release = Cluster::Release {
        reader: "resources".into(),
    };
    assert!(asked.contains(&Command::Cluster(release)), "{asked:?}");
    let watches = |one: &Command| matches!(one, Command::Cluster(Cluster::Watch { .. }));
    assert!(!asked.iter().any(watches), "{asked:?}");
    let tick = Command::Cluster(Cluster::Age { now });
    assert!(!asked.contains(&tick), "no tick for rows nobody sees");
    assert_eq!(crate::ages_due(&app, &ui), None, "and no wake for them");
}
