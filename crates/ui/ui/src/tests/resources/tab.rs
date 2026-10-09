//! An object's own tab: opened from its row, what it reads, and what it draws.

use groove_controllers::cluster::Command as Cluster;
use groove_controllers::cluster_service::Followed;
use groove_controllers::{AppState, Command};
use groove_types::{
    Condition, Container, Described, EventRow, FollowKey, Owner, PodPart, State, Timestamp, Usage,
};

use super::{kind, listing, pods};
use crate::Ui;
use crate::hit::Target;
use crate::tests::settings::drawn;
use crate::tests::{click, window};
use crate::views::session::resources::Link;

fn base(uid: &str, name: &str) -> Described {
    Described {
        uid: uid.into(),
        name: name.into(),
        namespace: Some("paxone".into()),
        labels: vec![("app".into(), "worker".into())],
        annotations: vec![("meta.helm.sh/release-name".into(), "paxone-staging".into())],
        created: None,
        owners: Vec::new(),
        replicas: None,
        selector: Vec::new(),
        pod: None,
        event: None,
        yaml: "".into(),
    }
}

fn worker() -> Container {
    Container {
        name: "worker".into(),
        image: "backend:4.21.3".into(),
        init: false,
        state: State::Waiting {
            reason: "CrashLoopBackOff".into(),
        },
        ready: false,
        restarts: 14,
        last: None,
        ports: vec!["http 8080/TCP".into()],
        liveness: None,
        readiness: None,
        mounts: vec!["tls → /etc/tls ro".into()],
        cpu: (Some("250m".into()), Some("1".into())),
        memory: (Some("512Mi".into()), Some("1Gi".into())),
    }
}

pub(super) fn crashing() -> Described {
    let pod = PodPart {
        status: "CrashLoopBackOff".into(),
        node: Some("gra9-node-2".into()),
        ip: Some("10.2.4.118".into()),
        qos: Some("Burstable".into()),
        priority: None,
        started: None,
        conditions: vec![Condition {
            kind: "Ready".into(),
            met: false,
            reason: Some("ContainersNotReady".into()),
            since: None,
        }],
        containers: vec![worker()],
        uses: vec![("secret".into(), "paxone-db-legacy".into())],
    };
    Described {
        owners: vec![Owner {
            api_version: "apps/v1".into(),
            kind: "ReplicaSet".into(),
            name: "worker-5d6b".into(),
        }],
        pod: Some(Box::new(pod)),
        yaml: "metadata:\n  name: api-0\nstatus:\n  restartCount: 14\n".into(),
        ..base("paxone/api-0", "api-0")
    }
}

/// The listing, its kinds widened to what a tab reads, and api-0's tab open.
pub(super) fn opened() -> (AppState, Ui) {
    let (mut app, mut ui) = listing();
    let kinds = vec![
        pods(),
        kind("apps", "ReplicaSet", "replicasets", true),
        kind("apps", "Deployment", "deployments", true),
        kind("", "Event", "events", true),
        kind("", "Service", "services", true),
        kind("", "Secret", "secrets", true),
    ];
    app.cluster.store.set_kinds("staging", kinds);
    let (_, hits) = drawn(&app, &ui);
    let row = hits
        .rect_of(&Target::ResourceRow("paxone/api-0".into()))
        .expect("api-0's row");
    click(row, &mut ui, &app, &hits);
    (app, ui)
}

pub(super) fn link(name: &str, of: groove_types::KubeKind) -> Link {
    Link {
        context: "staging".into(),
        kind: of,
        namespace: Some("paxone".into()),
        name: name.into(),
    }
}

/// The texts drawn on a wide screen, where the tab's columns have their room.
fn wide(app: &AppState, ui: &Ui) -> Vec<String> {
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
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

pub(super) fn lands(app: &mut AppState, key: &FollowKey, objects: Vec<Described>) {
    app.cluster.store.follows.lease(key, "resource");
    app.cluster
        .store
        .follows
        .apply(key, Followed::Reset(objects));
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
const DESCRIBED: [&str; 21] = [
    "CrashLoopBackOff",
    "waiting · CrashLoopBackOff",
    "worker",
    "14",
    "gra9-node-2",
    "CONDITIONS",
    "SUMMARY",
    "RELATIONS",
    "owned by",
    "worker-5d6b",
    "2/3",
    "uses",
    "paxone-db-legacy",
    "BackOff",
    "×187",
    "250m",
    "1",
    "180m",
    "700Mi",
    "http 8080/TCP",
    "tls → /etc/tls ro",
];

#[test]
fn a_pod_read_asks_its_events_owners_services_usage_and_release_and_draws_them() {
    let (mut app, ui) = opened();
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
    let texts = wide(&app, &ui);
    for shown in DESCRIBED {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
}

/// Every run drawn on a wide screen.
pub(super) fn runs(app: &AppState, ui: &Ui) -> Vec<groove_gfx::TextRun> {
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
    let (frame, _) = crate::view(app, ui, metrics, &mut groove_gfx::Fonts::embedded());
    frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.clone())
        .collect()
}

#[test]
fn the_gauges_keep_their_height_whatever_the_card_holds_and_uses_sit_by_their_names() {
    let (mut app, ui) = opened();
    let key = link("api-0", pods()).key();
    lands(&mut app, &key, vec![crashing()]);
    let at = |runs: &[groove_gfx::TextRun], text: &str| {
        runs.iter()
            .find(|one| one.text == text)
            .map(|one| (one.x, one.y))
            .expect(text)
    };
    let few = runs(&app, &ui);
    let mut many = crashing();
    if let Some(pod) = many.pod.as_mut() {
        pod.containers[0].mounts = (0..12).map(|one| format!("v{one} → /v{one}")).collect();
    }
    app.cluster
        .store
        .follows
        .apply(&key, Followed::Reset(vec![many]));
    let lots = runs(&app, &ui);
    assert_eq!(
        at(&few, "250m").1,
        at(&lots, "250m").1,
        "the request's tick stands still"
    );
    let (tag, name) = (at(&few, "secret"), at(&few, "paxone-db-legacy"));
    let styles = groove_ui_kit::base::style::Styles::new(app.config.theme(), window().tokens());
    let mut fonts = groove_gfx::Fonts::embedded();
    let style = styles.small(groove_ui_kit::base::style::Role::Ghost);
    let tag_wide = fonts.measure("secret", style.font, style.weight, style.size);
    assert!(
        name.0 - tag.0 <= tag_wide + window().tokens().md,
        "{} after {}",
        name.0,
        tag.0
    );
}

/// The wheel turned over `at`, on a wide screen.
pub(super) fn wheel(app: &AppState, ui: &mut Ui, at: (f32, f32), (across, down): (f32, f32)) {
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
    let (_, hits) = crate::view(app, ui, metrics, &mut groove_gfx::Fonts::embedded());
    let delta = crate::input::Delta::Pixels { across, down };
    let input = crate::input::Input::Scroll {
        x: at.0,
        y: at.1,
        delta,
    };
    crate::input::handle(input, ui, app, &hits, metrics);
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
    let at = |ui: &Ui, text: &str| {
        runs(&app, ui)
            .iter()
            .find(|one| one.text == text)
            .map(|one| one.y)
    };
    let (label, reason) = (at(&ui, "label-00"), at(&ui, "Reason39"));
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
    let (_, hits) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let middle = hits
        .rect_of(&Target::ResourceView(false))
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
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
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
