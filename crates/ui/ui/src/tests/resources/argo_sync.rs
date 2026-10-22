//! An Application's resources view: what needs attention first, by kind, and a sync's stages as it runs.

use groove_controllers::cluster_service::Followed;
use groove_controllers::{AppState, Command};
use groove_types::{AppResource, Described, SyncedResource};

use super::argo::{applications, failing, listed};
use super::fixture::runs;
use crate::Ui;
use crate::hit::Target;
use crate::tests::click;
use crate::views::session::resources::{Filter, Link, View};

pub(super) fn managed(kind: &str, name: &str, sync: &str, wave: i64) -> AppResource {
    AppResource {
        group: String::new(),
        kind: kind.into(),
        namespace: Some("billing-acc".into()),
        name: name.into(),
        sync: sync.into(),
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
        namespace: Some("billing-acc".into()),
        status: "Synced".into(),
        message: String::new(),
        hook,
        phase: if hook { "PreSync" } else { "Sync" }.into(),
        hook_phase: hook_phase.into(),
    }
}

/// The Application's tab open on its resources view.
pub(super) fn opened_application(object: Described) -> (AppState, Ui) {
    let (mut app, mut ui) = listed();
    let link = Link {
        context: "staging".into(),
        kind: applications(),
        namespace: Some("shop".into()),
        name: "billing-acc".into(),
    };
    app.cluster.store.follows.lease(&link.key(), "resource");
    let reset = Followed::Reset(vec![object]);
    app.cluster.store.follows.apply(&link.key(), reset);
    ui.session.resources.open(link);
    hit(&app, &mut ui, Target::ResourceView(View::Resources));
    (app, ui)
}

pub(super) fn hit(app: &AppState, ui: &mut Ui, target: Target) -> Vec<Command> {
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
    let operation = part.operation.as_mut().expect("its sync");
    operation.phase = "Failed".into();
    let mut broke = result("Job", "billing-migrate", false, "Failed");
    broke.status = "SyncFailed".into();
    broke.message = "field is immutable".into();
    operation.applied = vec![broke];
    part.resources = vec![
        managed("Deployment", "billing-api", "OutOfSync", 1),
        managed("Job", "billing-migrate", "Synced", -1),
        managed("ConfigMap", "billing-config", "Synced", 0),
        managed("Service", "billing", "Synced", 0),
    ];
    let (app, mut ui) = opened_application(object);
    let shown = texts(&app, &ui);
    for one in [
        "resources",
        "needs attention · 2",
        "out of sync · 1",
        "syncing · 0",
        "Deployment",
        "billing-api",
        "1 out of sync",
        "1 failed",
        "field is immutable",
        "billing-migrate",
    ] {
        assert!(shown.iter().any(|text| text == one), "{one}: {shown:?}");
    }
    assert!(!shown.iter().any(|text| text == "ConfigMap"), "{shown:?}");
    hit(&app, &mut ui, Target::ArgoFilter(Filter::All));
    let shown = texts(&app, &ui);
    assert!(shown.iter().any(|text| text == "ConfigMap"), "{shown:?}");
    assert!(
        !shown.iter().any(|text| text == "billing-config"),
        "a kind with nothing to see stands folded"
    );
    hit(
        &app,
        &mut ui,
        Target::ResourceFold("ConfigMap".into(), true),
    );
    let shown = texts(&app, &ui);
    assert!(
        shown.iter().any(|text| text == "billing-config"),
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
        result("Job", "billing-migrate", true, "Succeeded"),
        result("Deployment", "billing-api", false, "Running"),
    ];
    part.resources = vec![
        managed("Deployment", "billing-api", "OutOfSync", 0),
        managed("ConfigMap", "billing-config", "OutOfSync", 1),
    ];
    let (app, ui) = opened_application(object);
    let shown = texts(&app, &ui);
    for one in [
        "syncing · 2",
        "Syncing",
        "1 of 3",
        "PreSync hooks",
        "wave 0",
        "1 applying",
        "billing-api",
        "wave 1",
        "1 waiting",
        "billing-config",
    ] {
        assert!(shown.iter().any(|text| text == one), "{one}: {shown:?}");
    }
    assert!(
        !shown.iter().any(|text| text == "billing-migrate"),
        "the hooks done stand folded: {shown:?}"
    );
}

#[test]
fn a_header_sorts_each_kind_s_rows_and_leaves_the_list_s_order_be() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.resources = vec![
        managed("Deployment", "billing-api", "OutOfSync", 0),
        managed("Deployment", "billing-worker", "OutOfSync", 0),
    ];
    let (app, mut ui) = opened_application(object);
    let at = |app: &AppState, ui: &Ui, name: &str| {
        let drawn = runs(app, ui);
        drawn.iter().find(|one| one.text == name).map(|one| one.y)
    };
    let (api, worker) = (
        at(&app, &ui, "billing-api"),
        at(&app, &ui, "billing-worker"),
    );
    assert!(api < worker, "by name at first");
    hit(&app, &mut ui, Target::ResourceSort("NAME".into()));
    hit(&app, &mut ui, Target::ResourceSort("NAME".into()));
    let (api, worker) = (
        at(&app, &ui, "billing-api"),
        at(&app, &ui, "billing-worker"),
    );
    assert!(api > worker, "the second click turns the order round");
    let tab = ui.session.resources.tab().expect("its tab");
    assert_eq!(tab.sort, Some(("NAME".to_string(), true)));
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
        result("Job", "billing-migrate", true, "Succeeded"),
        result("Deployment", "billing-api", false, ""),
    ];
    let mut job = managed("Job", "billing-migrate", "OutOfSync", 0);
    job.hook = true;
    part.resources = vec![managed("Deployment", "billing-api", "Synced", 0), job];
    let (app, mut ui) = opened_application(object);
    let shown = texts(&app, &ui);
    assert!(
        shown.iter().any(|text| text == "out of sync · 0"),
        "{shown:?}"
    );
    hit(&app, &mut ui, Target::ArgoFilter(Filter::Syncing));
    hit(
        &app,
        &mut ui,
        Target::ResourceFold("PreSync hooks".into(), true),
    );
    hit(&app, &mut ui, Target::ResourceFold("wave 0".into(), true));
    let shown = texts(&app, &ui);
    assert!(
        shown.iter().any(|text| text == "billing-migrate"),
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
    let mut seed = result("Job", "billing-seed", true, "Running");
    seed.phase = "Sync".into();
    operation.applied = vec![
        result("Deployment", "billing-api", false, "Succeeded"),
        seed,
    ];
    let mut listed = managed("Job", "billing-seed", "Synced", 1);
    listed.hook = true;
    part.resources = vec![
        managed("Deployment", "billing-api", "Synced", 0),
        managed("ConfigMap", "billing-config", "OutOfSync", 1),
        listed,
    ];
    let (app, ui) = opened_application(object);
    let shown = texts(&app, &ui);
    assert!(!shown.iter().any(|text| text == "Sync hooks"), "{shown:?}");
    let at = |text: &str| {
        let drawn = runs(&app, &ui);
        drawn.iter().find(|one| one.text == text).map(|one| one.y)
    };
    let (wave, seed) = (at("wave 1"), at("billing-seed"));
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

#[test]
fn a_narrow_pane_keeps_the_application_s_badges_clear_of_its_views() {
    let (app, ui) = opened_application(failing());
    for width in [1100, 1300] {
        let metrics = crate::tests::metrics(width, 800, 1.0);
        let (frame, hits) = crate::view(&app, &ui, metrics, &mut groove_gfx::Fonts::embedded());
        let views = hits.rect_of(&Target::ResourceView(View::Describe));
        let views = views.expect("the views drawn");
        let mut texts = frame.layers().iter().flat_map(|layer| layer.texts.clone());
        let badge = texts.rfind(|one| one.text == "Degraded");
        let Some(badge) = badge else {
            assert_eq!(width, 1100, "the badges fit at {width}");
            continue;
        };
        assert!(
            badge.clip.x + badge.clip.w <= views.x,
            "at {width}: {:?} over {views:?}",
            badge.clip
        );
    }
}

#[test]
fn a_source_s_credentials_are_never_drawn() {
    let mut object = super::argo::failing();
    let part = object.app.as_mut().expect("its part");
    part.sources[0].repo = "https://ci:s3cr3t@gitlab.example.com/billing-eu-deploy.git".into();
    let (app, mut ui) = opened_application(object);
    ui.session.resources.opened[0].view = crate::views::session::resources::View::Describe;
    let shown = texts(&app, &ui);
    assert!(!shown.iter().any(|one| one.contains("s3cr3t")), "{shown:?}");
    assert!(
        shown
            .iter()
            .any(|one| one == "gitlab.example.com/billing-eu-deploy"),
        "{shown:?}"
    );
}
