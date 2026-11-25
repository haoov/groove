//! An Application's sync and refresh from its tab's heading: the sync asked over its diff.

use groove_controllers::Command;
use groove_controllers::cluster::Command as Cluster;

use super::argo::failing;
use super::argo_sync::{hit, opened_application};
use crate::hit::Target;
use crate::views::session::resources::{View, Write};
use crate::{Losing, Overlay};

fn app() -> (String, String, String) {
    ("staging".into(), "shop".into(), "billing-acc".into())
}

#[test]
fn refresh_runs_at_once_and_changes_nothing_asked() {
    let (app_state, mut ui) = opened_application(failing());
    let sent = hit(&app_state, &mut ui, Target::ArgoRefresh);
    let refresh = Command::Cluster(Cluster::ArgoRefresh {
        app: app(),
        hard: false,
    });
    assert_eq!(sent, [refresh]);
    assert!(ui.overlay.is_none(), "nothing asked");
}

#[test]
fn sync_is_asked_over_the_diff_prune_toggled_on_and_answered_on_the_resources() {
    let (app_state, mut ui) = opened_application(failing());
    hit(&app_state, &mut ui, Target::ArgoSync);
    let view = ui.session.resources.tab().map(|one| one.view);
    assert_eq!(view, Some(View::Diff), "the diff it would apply");
    let asked = |ui: &crate::Ui| match &ui.overlay {
        Some(Overlay::Losing(Losing::Row(written))) => Some(written.write),
        _ => None,
    };
    assert_eq!(asked(&ui), Some(Write::Sync { prune: false }));
    hit(&app_state, &mut ui, Target::ArgoPrune);
    assert_eq!(asked(&ui), Some(Write::Sync { prune: true }), "prune on");
    let sent = hit(&app_state, &mut ui, Target::Discard);
    let session = app_state.session.selected.clone().expect("a session");
    let sync = Command::Cluster(Cluster::ArgoSync {
        session,
        app: app(),
        prune: true,
    });
    assert_eq!(sent, [sync]);
    let view = ui.session.resources.tab().map(|one| one.view);
    assert_eq!(view, Some(View::Resources), "the sync followed there");
}
