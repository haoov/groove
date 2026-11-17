//! A resource's right-click: a pod's shell, a restart asked on its row, and the skills that act on its kind.

use super::fixture::opened;
use super::*;
use crate::input::Input;
use crate::tests::click;
use crate::{Losing, Of};

pub(super) fn skill(name: &str, resources: &[&str]) -> groove_types::Skill {
    groove_types::Skill {
        id: format!("groove:{name}"),
        plugin: "groove".into(),
        name: name.into(),
        description: "What it does.".into(),
        hint: String::new(),
        label: name.into(),
        kinds: Vec::new(),
        resources: resources.iter().map(|one| one.to_string()).collect(),
        editable: false,
        enabled: true,
        changed_at: groove_types::Timestamp::new(0),
    }
}

/// The menu a right click on `target` opens, if one does.
fn menu_on(app: &AppState, ui: &mut Ui, target: &Target) -> Option<Of> {
    let (_, hits) = drawn(app, ui);
    let rect = hits
        .rect_of(target)
        .unwrap_or_else(|| panic!("{target:?} was drawn"));
    let (x, y) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
    crate::input::handle(Input::Menu { x, y }, ui, app, &hits, window());
    ui.menu().map(|one| one.of.clone())
}

fn skilled(app: &mut AppState) {
    app.skill.skills = vec![
        skill("investigate", &["*"]),
        skill("rollout-check", &["deployment"]),
        skill("save-task", &[]),
    ];
}

const API_0: &str = "context: staging, namespace: shop, kind: Pod (pods), name: api-0";

#[test]
fn a_resource_row_offers_the_skills_of_its_kind_sent_with_the_resource() {
    let (mut app, mut ui) = listing();
    skilled(&mut app);
    let row = Target::ResourceRow("shop/api-0".into());
    let Some(Of::Resource { offered, .. }) = menu_on(&app, &mut ui, &row) else {
        panic!("a skills menu");
    };
    let labels: Vec<&str> = offered.iter().map(|one| one.label.as_str()).collect();
    assert_eq!(
        labels,
        ["investigate"],
        "no deployment's skill, no session's skill"
    );
    assert_eq!(offered[0].args.as_deref(), Some(API_0));
}

#[test]
fn an_object_tab_opens_no_menu() {
    let (mut app, mut ui) = opened();
    skilled(&mut app);
    let (_, hits) = drawn(&app, &ui);
    let rect = hits.rect_of(&Target::ResourceTab(0)).expect("the tab");
    let (x, y) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
    crate::input::handle(Input::Menu { x, y }, &mut ui, &app, &hits, window());
    assert!(ui.menu().is_none());
}

#[test]
fn a_pod_s_row_offers_its_shell_then_its_delete_asked_in_its_place() {
    let (mut app, mut ui) = listing();
    app.skill.skills = vec![skill("save-task", &[])];
    let row = Target::ResourceRow("shop/api-0".into());
    let Some(of) = menu_on(&app, &mut ui, &row) else {
        panic!("a menu on the pod");
    };
    let rows = crate::views::overlays::actions::rows(&of);
    assert_eq!(rows, ["shell", "delete"], "no scale, no restart, no skill");
    picked(&app, &mut ui, 1);
    let (texts, hits) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "delete api-0?"), "{texts:?}");
    let yes = hits.rect_of(&Target::Discard).expect("the delete answer");
    let sent = click(yes, &mut ui, &app, &hits);
    let [Command::Cluster(cluster::Command::Delete { target, .. })] = sent.as_slice() else {
        panic!("one delete: {sent:?}");
    };
    assert_eq!(target.label(), "staging · shop/pods/api-0");
}

#[test]
fn a_pod_s_shell_opens_the_drawer_with_the_keys_going_to_it() {
    let (app, mut ui) = listing();
    menu_on(&app, &mut ui, &Target::ResourceRow("shop/api-0".into()));
    let sent = picked(&app, &mut ui, 0);
    let [Command::Shell(groove_controllers::shell::Command::Pod { pod, container, .. })] =
        sent.as_slice()
    else {
        panic!("one shell: {sent:?}");
    };
    assert_eq!(pod.label(), "staging · shop/pods/api-0");
    assert!(container.is_none(), "kubectl picks the container");
    assert!(ui.session.manual, "the drawer is open");
    assert_eq!(ui.focus, crate::Focus::Terminal);
}

/// The listing turned to the deployments of `shop`: `api` alone, ready on one of two.
fn deployments() -> (AppState, Ui) {
    let (mut app, mut ui) = listing();
    let deployments = kind("apps", "Deployment", "deployments", true);
    let key = WatchKey {
        kind: deployments.clone(),
        ..key("staging", Some("shop"))
    };
    app.cluster.store.lease(&key, "resources");
    let mut api = row("api", "shop", "Running");
    (api.uid, api.cells[1]) = ("shop/api".into(), "1/2".into());
    let column = |name: &str| TableColumn {
        name: name.into(),
        priority: 0,
        date: false,
    };
    let (columns, rows) = (vec![column("Name"), column("Ready")], vec![api]);
    app.cluster
        .store
        .apply(&key, Batch::Reset { columns, rows });
    ui.session.resources.kind = Some(deployments);
    (app, ui)
}

/// A click on the open menu's row `at`.
fn picked(app: &AppState, ui: &mut Ui, at: usize) -> Vec<Command> {
    let (_, hits) = drawn(app, ui);
    let rect = hits.rect_of(&Target::MenuRow(at)).expect("the menu row");
    click(rect, ui, app, &hits)
}

#[test]
fn a_deployment_s_row_asks_its_restart_in_its_place_and_restart_sends_it() {
    let (app, mut ui) = deployments();
    let row = Target::ResourceRow("shop/api".into());
    let Some(of) = menu_on(&app, &mut ui, &row) else {
        panic!("a menu on the deployment");
    };
    let rows = crate::views::overlays::actions::rows(&of);
    assert_eq!(rows, ["scale", "restart", "delete"]);
    assert!(picked(&app, &mut ui, 1).is_empty(), "nothing written yet");
    assert!(matches!(ui.losing(), Some(Losing::Row(_))));
    let (texts, hits) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "restart api?"), "{texts:?}");
    let yes = hits.rect_of(&Target::Discard).expect("the restart answer");
    let sent = click(yes, &mut ui, &app, &hits);
    let [Command::Cluster(cluster::Command::Restart { session, target })] = sent.as_slice() else {
        panic!("one restart: {sent:?}");
    };
    assert_eq!(session.as_str(), "a");
    assert_eq!(target.label(), "staging · shop/deployments/api");
    assert!(ui.losing().is_none());
}

#[test]
fn keep_leaves_the_deployment_as_it_is() {
    let (app, mut ui) = deployments();
    menu_on(&app, &mut ui, &Target::ResourceRow("shop/api".into()));
    picked(&app, &mut ui, 0);
    let (_, hits) = drawn(&app, &ui);
    let keep = hits.rect_of(&Target::Keep).expect("the keep answer");
    assert!(click(keep, &mut ui, &app, &hits).is_empty());
    assert!(ui.losing().is_none());
}

#[test]
fn a_scale_starts_from_the_count_the_row_wants_and_steps_before_it_is_sent() {
    let (app, mut ui) = deployments();
    menu_on(&app, &mut ui, &Target::ResourceRow("shop/api".into()));
    picked(&app, &mut ui, 0);
    let (texts, _) = drawn(&app, &ui);
    for shown in ["scale api to", "2"] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    for by in [1, 1, -1] {
        let (_, hits) = drawn(&app, &ui);
        let step = hits.rect_of(&Target::Step(by)).expect("the step");
        assert!(click(step, &mut ui, &app, &hits).is_empty());
    }
    let (_, hits) = drawn(&app, &ui);
    let yes = hits.rect_of(&Target::Discard).expect("the scale answer");
    let sent = click(yes, &mut ui, &app, &hits);
    let [Command::Cluster(cluster::Command::Scale { replicas, .. })] = sent.as_slice() else {
        panic!("one scale: {sent:?}");
    };
    assert_eq!(*replicas, 3);
}

#[test]
fn a_scale_to_none_offers_no_step_down() {
    let (app, mut ui) = deployments();
    menu_on(&app, &mut ui, &Target::ResourceRow("shop/api".into()));
    picked(&app, &mut ui, 0);
    for _ in 0..2 {
        let (_, hits) = drawn(&app, &ui);
        let step = hits.rect_of(&Target::Step(-1)).expect("a step down");
        click(step, &mut ui, &app, &hits);
    }
    let (texts, hits) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "0"), "{texts:?}");
    assert!(hits.rect_of(&Target::Step(-1)).is_none());
}
