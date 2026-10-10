//! An Application's resources view: what needs attention first, by kind, and a sync's stages as it runs.

use groove_controllers::cluster_service::Followed;
use groove_controllers::{AppState, Command};
use groove_types::{AppResource, Described, SyncedResource};

use super::argo::{applications, failing, listed};
use super::tab::runs;
use crate::Ui;
use crate::hit::Target;
use crate::tests::click;
use crate::views::session::resources::{Filter, Link, View};

fn managed(kind: &str, name: &str, sync: &str, health: &str, wave: i64) -> AppResource {
    AppResource {
        group: String::new(),
        kind: kind.into(),
        namespace: Some("pythie-acc".into()),
        name: name.into(),
        sync: sync.into(),
        health: health.into(),
        message: String::new(),
        wave,
        prune: false,
        hook: false,
    }
}

fn result(kind: &str, name: &str, hook: bool, hook_phase: &str) -> SyncedResource {
    SyncedResource {
        group: String::new(),
        kind: kind.into(),
        name: name.into(),
        namespace: Some("pythie-acc".into()),
        status: "Synced".into(),
        message: String::new(),
        hook,
        phase: if hook { "PreSync" } else { "Sync" }.into(),
        hook_phase: hook_phase.into(),
    }
}

/// The Application's tab open on its resources view.
fn opened(object: Described) -> (AppState, Ui) {
    let (mut app, mut ui) = listed();
    let link = Link {
        context: "staging".into(),
        kind: applications(),
        namespace: Some("paxone".into()),
        name: "pythie-acc".into(),
    };
    app.cluster.store.follows.lease(&link.key(), "resource");
    let reset = Followed::Reset(vec![object]);
    app.cluster.store.follows.apply(&link.key(), reset);
    ui.session.resources.open(link);
    hit(&app, &mut ui, Target::ResourceView(View::Resources));
    (app, ui)
}

fn hit(app: &AppState, ui: &mut Ui, target: Target) -> Vec<Command> {
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
    let (_, hits) = crate::view(app, ui, metrics, &mut groove_gfx::Fonts::embedded());
    let at = hits.rect_of(&target);
    let at = at.unwrap_or_else(|| panic!("{target:?} drawn"));
    click(at, ui, app, &hits)
}

fn texts(app: &AppState, ui: &Ui) -> Vec<String> {
    runs(app, ui).into_iter().map(|one| one.text).collect()
}

#[test]
fn resources_open_on_what_needs_attention_and_all_folds_the_healthy_kinds() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.operation.as_mut().expect("its sync").phase = "Succeeded".into();
    part.resources = vec![
        managed("Deployment", "pythie-api", "OutOfSync", "Progressing", 1),
        managed("Job", "pythie-migrate", "Synced", "Degraded", -1),
        managed("ConfigMap", "pythie-config", "Synced", "", 0),
        managed("Service", "pythie", "Synced", "Healthy", 0),
    ];
    let (app, mut ui) = opened(object);
    let shown = texts(&app, &ui);
    for one in [
        "resources",
        "needs attention · 2",
        "out of sync · 1",
        "unhealthy · 2",
        "syncing · 0",
        "Deployment",
        "pythie-api",
        "1 out of sync · 1 progressing",
        "pythie-migrate",
    ] {
        assert!(shown.iter().any(|text| text == one), "{one}: {shown:?}");
    }
    assert!(!shown.iter().any(|text| text == "ConfigMap"), "{shown:?}");
    hit(&app, &mut ui, Target::ArgoFilter(Filter::All));
    let shown = texts(&app, &ui);
    assert!(shown.iter().any(|text| text == "ConfigMap"), "{shown:?}");
    assert!(
        !shown.iter().any(|text| text == "pythie-config"),
        "a kind with nothing to see stands folded"
    );
    hit(&app, &mut ui, Target::ArgoFold("ConfigMap".into()));
    let shown = texts(&app, &ui);
    assert!(
        shown.iter().any(|text| text == "pythie-config"),
        "{shown:?}"
    );
}

#[test]
fn a_running_sync_opens_on_its_stages_and_counts_what_it_has_applied() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    let operation = part.operation.as_mut().expect("its sync");
    operation.phase = "Running".into();
    operation.failed.clear();
    operation.applied = vec![
        result("Job", "pythie-migrate", true, "Succeeded"),
        result("Deployment", "pythie-api", false, "Running"),
    ];
    part.resources = vec![
        managed("Deployment", "pythie-api", "OutOfSync", "", 0),
        managed("ConfigMap", "pythie-config", "OutOfSync", "", 1),
    ];
    let (app, ui) = opened(object);
    let shown = texts(&app, &ui);
    for one in [
        "syncing · 2",
        "Syncing",
        "1 of 3",
        "PreSync hooks",
        "wave 0",
        "1 applying",
        "pythie-api",
        "wave 1",
        "1 waiting",
        "pythie-config",
    ] {
        assert!(shown.iter().any(|text| text == one), "{one}: {shown:?}");
    }
    assert!(
        !shown.iter().any(|text| text == "pythie-migrate"),
        "the hooks done stand folded: {shown:?}"
    );
}

#[test]
fn a_header_sorts_each_kind_s_rows_and_leaves_the_list_s_order_be() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.resources = vec![
        managed("Deployment", "pythie-api", "OutOfSync", "Healthy", 0),
        managed("Deployment", "pythie-worker", "OutOfSync", "Healthy", 0),
    ];
    let (app, mut ui) = opened(object);
    let at = |app: &AppState, ui: &Ui, name: &str| {
        let drawn = runs(app, ui);
        drawn.iter().find(|one| one.text == name).map(|one| one.y)
    };
    let (api, worker) = (at(&app, &ui, "pythie-api"), at(&app, &ui, "pythie-worker"));
    assert!(api < worker, "by name at first");
    hit(&app, &mut ui, Target::ResourceSort("NAME".into()));
    hit(&app, &mut ui, Target::ResourceSort("NAME".into()));
    let (api, worker) = (at(&app, &ui, "pythie-api"), at(&app, &ui, "pythie-worker"));
    assert!(api > worker, "the second click turns the order round");
    let tab = ui.session.resources.tab().expect("its tab");
    assert_eq!(tab.argo.sort, Some(("NAME".to_string(), true)));
    assert_eq!(
        ui.session.resources.sort, None,
        "the list keeps its own order"
    );
}

#[test]
fn a_hook_job_left_after_its_sync_is_neither_out_of_sync_nor_skipped() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    let operation = part.operation.as_mut().expect("its sync");
    operation.phase = "Succeeded".into();
    operation.failed.clear();
    operation.applied = vec![
        result("Job", "pythie-migrate", true, "Succeeded"),
        result("Deployment", "pythie-api", false, ""),
    ];
    let mut job = managed("Job", "pythie-migrate", "OutOfSync", "Healthy", 0);
    job.hook = true;
    part.resources = vec![
        managed("Deployment", "pythie-api", "Synced", "Healthy", 0),
        job,
    ];
    let (app, mut ui) = opened(object);
    let shown = texts(&app, &ui);
    assert!(
        shown.iter().any(|text| text == "out of sync · 0"),
        "{shown:?}"
    );
    hit(&app, &mut ui, Target::ArgoFilter(Filter::Syncing));
    hit(&app, &mut ui, Target::ArgoFold("PreSync hooks".into()));
    hit(&app, &mut ui, Target::ArgoFold("wave 0".into()));
    let shown = texts(&app, &ui);
    assert!(
        shown.iter().any(|text| text == "pythie-migrate"),
        "{shown:?}"
    );
    assert!(shown.iter().any(|text| text == "1 applied"), "{shown:?}");
    assert!(
        !shown.iter().any(|text| text.contains("not in this sync")),
        "the hook counts once, in its phase: {shown:?}"
    );
}

#[test]
fn a_sync_hook_runs_in_the_wave_the_application_lists_it_at() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    let operation = part.operation.as_mut().expect("its sync");
    operation.phase = "Running".into();
    operation.failed.clear();
    let mut seed = result("Job", "pythie-seed", true, "Running");
    seed.phase = "Sync".into();
    operation.applied = vec![result("Deployment", "pythie-api", false, "Succeeded"), seed];
    let mut listed = managed("Job", "pythie-seed", "Synced", "", 1);
    listed.hook = true;
    part.resources = vec![
        managed("Deployment", "pythie-api", "Synced", "", 0),
        managed("ConfigMap", "pythie-config", "OutOfSync", "", 1),
        listed,
    ];
    let (app, ui) = opened(object);
    let shown = texts(&app, &ui);
    assert!(!shown.iter().any(|text| text == "Sync hooks"), "{shown:?}");
    let at = |text: &str| {
        let drawn = runs(&app, &ui);
        drawn.iter().find(|one| one.text == text).map(|one| one.y)
    };
    let (wave, seed) = (at("wave 1"), at("pythie-seed"));
    assert!(
        wave.is_some() && seed > wave,
        "the hook under wave 1: {shown:?}"
    );
    assert!(
        shown.iter().any(|text| text == "1 applying · 1 waiting"),
        "{shown:?}"
    );
    assert!(
        shown.iter().any(|text| text == "Sync Hook"),
        "its badge: {shown:?}"
    );
}
