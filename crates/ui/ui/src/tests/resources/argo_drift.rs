//! An Application's diff view: its drift read from Argo CD and drawn as the diff draws a change.

use groove_controllers::Command;
use groove_controllers::workspace::Command as Workspace;
use groove_controllers::workspace_service::prepare;

use super::argo::failing;
use super::argo_sync::{hit, opened_application};
use super::fixture::runs;
use crate::hit::Target;
use crate::tests::window;
use crate::views::session::resources::View;

fn key() -> (String, String, String) {
    ("staging".into(), "shop".into(), "billing-acc".into())
}

fn texts(app: &groove_controllers::AppState, ui: &crate::Ui) -> Vec<String> {
    runs(app, ui).into_iter().map(|one| one.text).collect()
}

#[test]
fn the_diff_view_reads_the_drift_and_draws_it_as_a_change() {
    let (mut app, mut ui) = opened_application(failing());
    hit(&app, &mut ui, Target::ResourceView(View::Diff));
    let asked = crate::frame_commands(&app, &ui, window());
    let show = Command::Workspace(Workspace::ShowDrift {
        app: key(),
        at: "1".into(),
    });
    assert!(asked.contains(&show), "{asked:?}");
    assert!(
        texts(&app, &ui)
            .iter()
            .any(|one| one.starts_with("reading the drift"))
    );

    let path = "argo/Deployment/billing-acc/billing-api.yaml".to_string();
    let pairs = vec![(path, "replicas: 2\n".into(), "replicas: 3\n".into())];
    app.workspace.drifted((key(), "1".into()), prepare(pairs));
    let shown = texts(&app, &ui);
    let head = "argo/Deployment/billing-acc/billing-api.yaml";
    assert!(shown.iter().any(|text| text == head), "{shown:?}");
    let runs = |run: &str| shown.iter().filter(|text| *text == run).count();
    assert_eq!(runs("replicas"), 2, "both sides drawn: {shown:?}");
    assert!(runs("2") >= 1 && runs("3") >= 1, "{shown:?}");
    let asked = crate::frame_commands(&app, &ui, window());
    assert!(
        !asked.contains(&show),
        "a drift shown at its version is not read again"
    );

    hit(&app, &mut ui, Target::ResourceView(View::Resources));
    let asked = crate::frame_commands(&app, &ui, window());
    assert!(
        asked.contains(&Command::Workspace(Workspace::LeaveDrift)),
        "{asked:?}"
    );
}

#[test]
fn an_application_with_nothing_differing_says_it_is_in_sync() {
    let (mut app, mut ui) = opened_application(failing());
    hit(&app, &mut ui, Target::ResourceView(View::Diff));
    app.workspace
        .drifted((key(), "1".into()), prepare(Vec::new()));
    let shown = texts(&app, &ui);
    assert!(
        shown.iter().any(|one| one.starts_with("in sync")),
        "{shown:?}"
    );
}

/// Deployments served on the Application's context, so its rows link to their objects.
fn served(app: &mut groove_controllers::AppState) -> groove_types::KubeKind {
    let deployments = super::kind("apps", "Deployment", "deployments", true);
    let mut kinds = app.cluster.store.kinds("staging").expect("kinds").to_vec();
    kinds.push(deployments);
    app.cluster.store.set_kinds("staging", kinds);
    let served = app.cluster.store.served("staging", "apps", "Deployment");
    served.expect("deployments served")
}

fn deployed(name: &str) -> groove_types::AppResource {
    let mut one = super::argo_sync::managed("Deployment", name, "OutOfSync", 0);
    one.group = "apps".into();
    one
}

/// The Application with two Deployments out of sync, its tab open, Deployments served.
fn drifting() -> (
    groove_controllers::AppState,
    crate::Ui,
    groove_types::KubeKind,
) {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.operation = None;
    part.destination.server = Some("https://kubernetes.default.svc".into());
    part.resources = vec![deployed("billing-api"), deployed("billing-web")];
    let (mut app, ui) = opened_application(object);
    let kind = served(&mut app);
    (app, ui, kind)
}

fn row(kind: &groove_types::KubeKind, name: &str) -> Target {
    let link = crate::views::session::resources::Link {
        context: "staging".into(),
        kind: kind.clone(),
        namespace: Some("billing-acc".into()),
        name: name.into(),
    };
    let path = groove_types::drift_path("Deployment", Some("billing-acc"), name);
    Target::ResourceDrift(path, Some(Box::new(link)))
}

#[test]
fn a_row_out_of_sync_opens_its_drift_and_lands_on_its_file() {
    let (mut app, mut ui, kind) = drifting();
    hit(&app, &mut ui, row(&kind, "billing-web"));
    let tab = ui.session.resources.tab().expect("its tab");
    assert_eq!(tab.view, View::Diff);
    let pairs = ["billing-api", "billing-web"].map(|name| {
        let path = groove_types::drift_path("Deployment", Some("billing-acc"), name);
        (
            path,
            "replicas: 2\n".to_string(),
            "replicas: 3\n".to_string(),
        )
    });
    app.workspace
        .drifted((key(), "1".into()), prepare(pairs.to_vec()));
    crate::input::reveal(&mut ui, &app, window());
    assert!(
        ui.session.revealing.is_none(),
        "the file waited for is reached"
    );
    assert!(
        ui.session.diff > 0.0,
        "the stream stands on the second file"
    );
}

#[test]
fn a_row_of_the_application_offers_the_skills_of_its_kind() {
    let (mut app, mut ui, kind) = drifting();
    app.skill.skills = vec![super::skilled::skill("investigate", &["*"])];
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
    let (_, hits) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
    let rect = hits.rect_of(&row(&kind, "billing-api")).expect("its row");
    let (x, y) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
    crate::input::handle(
        crate::input::Input::Menu { x, y },
        &mut ui,
        &app,
        &hits,
        window(),
    );
    let Some(crate::Of::Skills { offered, .. }) = ui.menu().map(|one| one.of.clone()) else {
        panic!("a skills menu");
    };
    assert_eq!(offered[0].id, "groove:investigate");
    let told = offered[0].args.as_deref().unwrap_or_default();
    assert!(told.contains("name: billing-api"), "{told}");
}
