//! The fixtures the Resources tests share: a crashing pod, its tab open, and the wheel over it.

use groove_controllers::AppState;
use groove_controllers::cluster_service::Followed;
use groove_types::{Condition, Container, Described, FollowKey, Owner, PodPart, State};

use super::{kind, listing, pods};
use crate::Ui;
use crate::hit::Target;
use crate::tests::click;
use crate::tests::settings::drawn;
use crate::views::session::resources::Link;

pub(super) fn base(uid: &str, name: &str) -> Described {
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
        app: None,
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
        uses: vec![groove_types::Used {
            tag: "secret".into(),
            name: "paxone-db-legacy".into(),
            how: vec!["env worker".into()],
        }],
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

/// The events unfolded by a click on their heading, as they stand folded at first.
pub(super) fn unfold_events(app: &AppState, ui: &mut Ui) {
    unfold(app, ui, crate::views::session::resources::Section::Events);
}

/// `section` unfolded by a click on its heading.
pub(super) fn unfold(
    app: &AppState,
    ui: &mut Ui,
    section: crate::views::session::resources::Section,
) {
    let section = Target::ResourceSection(section);
    let metrics = crate::tests::metrics(1920, 1440, 1.0);
    let (_, hits) = crate::view(app, ui, metrics, &mut groove_gfx::Fonts::embedded());
    let heading = hits.rect_of(&section).expect("the events' heading");
    click(heading, ui, app, &hits);
}

pub(super) fn lands(app: &mut AppState, key: &FollowKey, objects: Vec<Described>) {
    app.cluster.store.follows.lease(key, "resource");
    app.cluster
        .store
        .follows
        .apply(key, Followed::Reset(objects));
}

/// Every run drawn on a wide screen.
pub(super) fn runs(app: &AppState, ui: &Ui) -> Vec<groove_gfx::TextRun> {
    let metrics = crate::tests::metrics(1920, 1440, 1.0);
    let (frame, _) = crate::view(app, ui, metrics, &mut groove_gfx::Fonts::embedded());
    frame
        .layers()
        .iter()
        .flat_map(|layer| layer.texts.clone())
        .collect()
}

/// The wheel turned over `at`, on a wide screen.
pub(super) fn wheel(app: &AppState, ui: &mut Ui, at: (f32, f32), (across, down): (f32, f32)) {
    let metrics = crate::tests::metrics(1920, 1440, 1.0);
    let (_, hits) = crate::view(app, ui, metrics, &mut groove_gfx::Fonts::embedded());
    let delta = crate::input::Delta::Pixels { across, down };
    let input = crate::input::Input::Scroll {
        x: at.0,
        y: at.1,
        delta,
    };
    crate::input::handle(input, ui, app, &hits, metrics);
}
