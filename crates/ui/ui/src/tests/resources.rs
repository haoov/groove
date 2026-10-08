//! The Resources tab: what it lists, the scope it reads, and the watchers it asks for.

use groove_controllers::cluster;
use groove_controllers::cluster_service::Batch;
use groove_controllers::{AppState, Command};
use groove_types::{Attached, KubeKind, ObjectRow, SessionId, TableColumn, WatchKey};

use super::settings::drawn;
use super::*;
use crate::hit::{Picks, Target};
use crate::views::session::Tab;

mod list;

pub(super) fn kind(group: &str, name: &str, plural: &str, namespaced: bool) -> KubeKind {
    KubeKind {
        group: group.into(),
        version: "v1".into(),
        kind: name.into(),
        plural: plural.into(),
        namespaced,
        watchable: true,
    }
}

pub(super) fn pods() -> KubeKind {
    kind("", "Pod", "pods", true)
}

pub(super) fn on(context: &str, namespace: Option<&str>) -> Attached {
    Attached {
        context: context.into(),
        namespace: namespace.map(str::to_string),
    }
}

pub(super) fn key(context: &str, namespace: Option<&str>) -> WatchKey {
    WatchKey {
        context: context.into(),
        kind: pods(),
        namespace: namespace.map(str::to_string),
        selector: None,
    }
}

pub(super) fn row(name: &str, namespace: &str, status: &str) -> ObjectRow {
    ObjectRow {
        uid: format!("{namespace}/{name}"),
        name: name.into(),
        namespace: Some(namespace.into()),
        version: "1".into(),
        cells: vec![name.into(), "1/1".into(), status.into(), "10.0.0.1".into()],
        aging: Vec::new(),
    }
}

/// The session holds staging on `paxone` and `cnpg`; each context's kinds known, two lists landed.
pub(super) fn listing() -> (AppState, Ui) {
    let mut app = full_app();
    let config = r#"{ "git": { "worktree_root": "~/code" } }"#;
    app.config.config = Some(serde_json::from_str(config).expect("a config"));
    app.config.add_cluster("staging");
    let open = app
        .session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session");
    open.clusters = vec![on("staging", Some("paxone")), on("staging", Some("cnpg"))];
    let kinds = vec![
        pods(),
        kind("apps", "Deployment", "deployments", true),
        kind("", "Node", "nodes", false),
    ];
    app.cluster.store.set_kinds("staging", kinds);
    let columns = ["Name", "Ready", "Status", "IP"]
        .iter()
        .zip([0, 0, 0, 1])
        .map(|(name, priority)| TableColumn {
            name: (*name).into(),
            priority,
            date: false,
        })
        .collect::<Vec<_>>();
    let lists = [
        (
            "paxone",
            vec![
                row("api-0", "paxone", "Running"),
                row("worker-0", "paxone", "CrashLoopBackOff"),
            ],
        ),
        ("cnpg", vec![row("cnpg-1", "cnpg", "Running")]),
    ];
    for (namespace, rows) in lists {
        let key = key("staging", Some(namespace));
        app.cluster.store.lease(&key, "resources");
        let columns = columns.clone();
        app.cluster
            .store
            .apply(&key, Batch::Reset { columns, rows });
    }
    let mut ui = Ui::default();
    ui.session.tab = Tab::Resources;
    (app, ui)
}

#[test]
fn the_tab_stands_only_while_the_session_holds_a_cluster() {
    let (app, ui) = listing();
    let (_, hits) = drawn(&app, &ui);
    assert!(hits.rect_of(&Target::Tab(Tab::Resources)).is_some());
    let (_, hits) = drawn(&full_app(), &Ui::default());
    assert!(hits.rect_of(&Target::Tab(Tab::Resources)).is_none());
}

#[test]
fn the_list_tab_names_the_kind_and_its_count_over_rows_told_apart_by_namespace() {
    let (app, ui) = listing();
    let (texts, _) = drawn(&app, &ui);
    for shown in [
        "Pods · 3",
        "NAMESPACE",
        "NAME",
        "STATUS",
        "paxone",
        "cnpg",
        "worker-0",
        "CrashLoopBackOff",
    ] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    assert!(
        !texts.iter().any(|one| one == "IP"),
        "a wide column stays out: {texts:?}"
    );
    for heading in [
        "WORKLOADS",
        "Deployments",
        "CLUSTER",
        "Nodes",
        "all namespaces",
    ] {
        assert!(
            texts.iter().any(|one| one == heading),
            "{heading}: {texts:?}"
        );
    }
}

#[test]
fn a_name_word_narrows_the_rows_and_a_label_word_asks_the_server_for_its_own_watchers() {
    let (app, mut ui) = listing();
    ui.session.resources.search.set("wkr-0");
    let (texts, _) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "Pods · 1"), "{texts:?}");
    ui.session.resources.search.set("app=api");
    let asked = crate::render::frame_commands(&app, &ui, window());
    let selected = asked.iter().filter_map(|one| match one {
        Command::Cluster(cluster::Command::Watch { key, .. }) => key.selector.as_deref(),
        _ => None,
    });
    assert_eq!(selected.collect::<Vec<_>>(), ["app=api", "app=api"]);
}

/// The cluster commands of one frame; the frame's other parts left out.
fn clustered(app: &AppState, ui: &Ui) -> Vec<Command> {
    let asked = crate::render::frame_commands(app, ui, window());
    asked
        .into_iter()
        .filter(|one| matches!(one, Command::Cluster(_)))
        .collect()
}

#[test]
fn the_tab_asks_a_watcher_a_namespace_and_the_kinds_it_lacks_and_lets_go_once_left() {
    let (mut app, ui) = listing();
    assert!(
        clustered(&app, &ui).is_empty(),
        "the watchers held are the ones wanted"
    );
    app.config.add_cluster("hub");
    let open = app
        .session
        .get_mut(&SessionId::new("a"))
        .expect("the session");
    open.clusters.push(on("hub", None));
    let asked = clustered(&app, &ui);
    let discover = cluster::Command::Discover {
        context: "hub".into(),
        again: false,
    };
    assert!(asked.contains(&Command::Cluster(discover)), "{asked:?}");
    let left = Ui::default();
    let asked = clustered(&app, &left);
    let release = cluster::Command::Release {
        reader: "resources".into(),
    };
    assert_eq!(asked, [Command::Cluster(release)]);
}

#[test]
fn a_scope_picker_s_menu_stays_open_as_rows_are_switched_and_the_list_follows() {
    let (app, mut ui) = listing();
    let (_, hits) = drawn(&app, &ui);
    let picker = hits
        .rect_of(&Target::Picker(Picks::Namespaces))
        .expect("the namespaces picker");
    click(picker, &mut ui, &app, &hits);
    let (_, hits) = drawn(&app, &ui);
    let first = hits.rect_of(&Target::MenuRow(0)).expect("a row for paxone");
    click(first, &mut ui, &app, &hits);
    assert!(ui.menu().is_some(), "the menu stays open");
    let (texts, _) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "Pods · 1"), "{texts:?}");
    let asked = crate::render::frame_commands(&app, &ui, window());
    let watched: Vec<_> = asked
        .iter()
        .filter_map(|one| match one {
            Command::Cluster(cluster::Command::Watch { key, .. }) => key.namespace.as_deref(),
            _ => None,
        })
        .collect();
    assert_eq!(watched, ["cnpg"]);
}

#[test]
fn a_kind_in_the_sidebar_is_listed_from_the_top() {
    let (app, mut ui) = listing();
    ui.session.resources.scroll = 40.0;
    let (_, hits) = drawn(&app, &ui);
    let deployments = kind("apps", "Deployment", "deployments", true);
    let at = hits
        .rect_of(&Target::ResourceKind(deployments.clone()))
        .expect("the deployments row");
    click(at, &mut ui, &app, &hits);
    assert_eq!(ui.session.resources.kind, Some(deployments));
    assert_eq!(ui.session.resources.scroll, 0.0);
}

#[test]
fn the_find_key_opens_the_list_s_bar_and_esc_shuts_it_while_empty() {
    let (app, mut ui) = listing();
    clicked_in(&app, &mut ui);
    let ctrl = crate::input::Modifiers {
        ctrl: true,
        ..Default::default()
    };
    press(crate::input::Key::Char('f'), ctrl, &mut ui, &app);
    assert!(ui.session.resources.finding && ui.session.resources.typing);
    for c in "api".chars() {
        press(
            crate::input::Key::Char(c),
            Default::default(),
            &mut ui,
            &app,
        );
    }
    let (texts, _) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "Pods · 1"), "{texts:?}");
    press(crate::input::Key::Escape, Default::default(), &mut ui, &app);
    assert!(ui.session.resources.finding, "a bar with words stays");
    ui.session.resources.search.clear();
    ui.session.resources.typing = true;
    press(crate::input::Key::Escape, Default::default(), &mut ui, &app);
    assert!(!ui.session.resources.finding);
}

#[test]
fn the_sidebar_s_search_narrows_the_kinds_and_a_heading_folds_its_own() {
    let (app, mut ui) = listing();
    ui.session.resources.filter.set("node");
    let (texts, _) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "Nodes"), "{texts:?}");
    assert!(!texts.iter().any(|one| one == "Deployments"), "{texts:?}");
    ui.session.resources.filter.clear();
    let (_, hits) = drawn(&app, &ui);
    let heading = hits.rect_of(&Target::ResourceGroup(groove_types::KindHeading::Builtin(
        groove_types::Builtin::Workloads,
    )));
    click(
        heading.expect("the workloads heading"),
        &mut ui,
        &app,
        &hits,
    );
    let (texts, _) = drawn(&app, &ui);
    assert!(!texts.iter().any(|one| one == "Deployments"), "{texts:?}");
    assert!(texts.iter().any(|one| one == "Nodes"), "{texts:?}");
}

#[test]
fn a_lone_context_picked_wears_its_hue_in_its_picker() {
    use groove_ui_kit::base::style::{Role, Styles};
    let (app, ui) = listing();
    let (frame, _) = crate::view(&app, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let styles = Styles::new(app.config.theme(), window().tokens());
    let hue = app
        .config
        .cluster("staging")
        .map(|one| one.hue)
        .expect("staging is added");
    let runs = frame.layers().iter().flat_map(|layer| layer.texts.iter());
    let picked = runs
        .filter(|one| one.text == "staging")
        .map(|one| one.style.color)
        .next();
    assert_eq!(picked, Some(styles.color(Role::Hue(hue))));
}

#[test]
fn a_crd_stands_under_its_own_group_and_the_rbac_kinds_under_theirs() {
    let (mut app, ui) = listing();
    let mut kinds = app.cluster.store.kinds("staging").expect("known").to_vec();
    kinds.push(kind("postgresql.cnpg.io", "Cluster", "clusters", true));
    kinds.push(kind(
        "rbac.authorization.k8s.io",
        "ClusterRole",
        "clusterroles",
        false,
    ));
    app.cluster.store.set_kinds("staging", kinds);
    let (texts, _) = drawn(&app, &ui);
    for shown in ["POSTGRESQL.CNPG.IO", "Clusters", "RBAC", "Clusterroles"] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    let at = |label: &str| texts.iter().position(|one| one == label);
    assert!(at("RBAC") < at("POSTGRESQL.CNPG.IO") && at("POSTGRESQL.CNPG.IO") < at("CLUSTER"));
}

#[test]
fn ctrl_shift_p_searches_the_kinds_by_name_group_or_group_and_kind() {
    let (mut app, mut ui) = listing();
    let mut kinds = app.cluster.store.kinds("staging").expect("known").to_vec();
    kinds.push(kind("networking.k8s.io", "Ingress", "ingresses", true));
    kinds.push(kind("rbac.authorization.k8s.io", "Role", "roles", true));
    kinds.push(kind(
        "rbac.authorization.k8s.io",
        "RoleBinding",
        "rolebindings",
        true,
    ));
    app.cluster.store.set_kinds("staging", kinds);
    clicked_in(&app, &mut ui);
    let both = crate::input::Modifiers {
        ctrl: true,
        shift: true,
        ..Default::default()
    };
    press(crate::input::Key::Char('p'), both, &mut ui, &app);
    assert!(ui.session.resources.filtering && !ui.session.resources.typing);
    let shown = |app: &AppState, ui: &Ui| {
        let (texts, _) = drawn(app, ui);
        let kinds = [
            "Pods",
            "Deployments",
            "Nodes",
            "Ingresses",
            "Roles",
            "Rolebindings",
        ];
        texts
            .into_iter()
            .filter(|one| kinds.contains(&one.as_str()))
            .collect::<Vec<_>>()
    };
    ui.session.resources.filter.set("networking");
    assert_eq!(shown(&app, &ui), ["Ingresses"]);
    ui.session.resources.filter.set("rbac/binding");
    assert_eq!(shown(&app, &ui), ["Rolebindings"]);
    ui.session.resources.filter.set("rbac/");
    assert_eq!(shown(&app, &ui), ["Roles", "Rolebindings"]);
}

/// The focus where a session opens it, then a click on the Resources tab, as the user gives it.
fn clicked_in(app: &AppState, ui: &mut Ui) {
    ui.focus = crate::Focus::Agent;
    let (_, hits) = drawn(app, ui);
    let tab = hits
        .rect_of(&Target::Tab(Tab::Resources))
        .expect("the resources tab");
    click(tab, ui, app, &hits);
}

#[test]
fn a_click_on_the_sidebar_s_bar_and_typing_narrow_the_kinds_and_leave_the_rows_be() {
    let (app, mut ui) = listing();
    ui.focus = crate::Focus::Agent;
    let (_, hits) = drawn(&app, &ui);
    let bar = hits
        .rect_of(&Target::ResourceFilter)
        .expect("the sidebar's bar");
    click(bar, &mut ui, &app, &hits);
    for c in "node".chars() {
        press(
            crate::input::Key::Char(c),
            Default::default(),
            &mut ui,
            &app,
        );
    }
    assert_eq!(ui.session.resources.filter.text(), "node");
    assert!(ui.session.resources.search.is_empty());
    let (texts, _) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "Pods · 3"), "{texts:?}");
    assert!(texts.iter().any(|one| one == "Nodes"), "{texts:?}");
    assert!(!texts.iter().any(|one| one == "Deployments"), "{texts:?}");
}

#[test]
fn in_the_kind_search_the_arrows_step_through_the_lines_enter_folds_a_heading_or_lists_a_kind() {
    use crate::views::session::resources::{Line, shown};
    let (app, mut ui) = listing();
    clicked_in(&app, &mut ui);
    let both = crate::input::Modifiers {
        ctrl: true,
        shift: true,
        ..Default::default()
    };
    let key = |key, ui: &mut Ui| {
        press(key, Default::default(), ui, &app);
    };
    press(crate::input::Key::Char('p'), both, &mut ui, &app);
    key(crate::input::Key::Char('d'), &mut ui);
    let lines = shown(&app, &ui);
    let stood = |ui: &Ui| ui.session.resources.cursor.map(|at| lines[at].clone());
    assert!(
        matches!(stood(&ui), Some(Line::Kind(_))),
        "typing stands on the first kind"
    );
    key(crate::input::Key::Up, &mut ui);
    let Some(Line::Heading(heading, _)) = stood(&ui) else {
        panic!("a heading above it: {:?}", stood(&ui));
    };
    key(crate::input::Key::Enter, &mut ui);
    assert!(ui.session.resources.folded.contains(&heading) && ui.session.resources.filtering);
    let under = shown(&app, &ui);
    assert!(
        under.len() < lines.len(),
        "the folded heading's kinds are gone"
    );
    key(crate::input::Key::Enter, &mut ui);
    assert!(
        !ui.session.resources.folded.contains(&heading),
        "Enter again opens it"
    );
    key(crate::input::Key::Down, &mut ui);
    let Some(Line::Kind(kind)) = stood(&ui) else {
        panic!("a kind below it: {:?}", stood(&ui));
    };
    key(crate::input::Key::Enter, &mut ui);
    assert_eq!(ui.session.resources.kind, Some(kind));
    assert!(!ui.session.resources.filtering);
}

#[test]
fn the_kind_listed_stands_on_the_held_ground_between_the_rules() {
    use groove_ui_kit::base::style::Styles;
    let (app, ui) = listing();
    let (frame, _) = crate::view(&app, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let held = Styles::new(app.config.theme(), window().tokens()).held();
    let quads = frame.layers().iter().flat_map(|layer| layer.quads.iter());
    assert!(
        quads.into_iter().any(|one| one.color == held),
        "a quad on the held ground"
    );
}

#[test]
fn a_failing_watch_says_the_list_is_not_live_over_the_rows_it_holds() {
    let (mut app, ui) = listing();
    let key = key("staging", Some("paxone"));
    app.cluster
        .store
        .apply(&key, Batch::Failed("406 Not Acceptable".into()));
    let (texts, _) = drawn(&app, &ui);
    let said = "not live: 406 Not Acceptable · trying again";
    assert!(texts.iter().any(|one| one == said), "{texts:?}");
    assert!(
        texts.iter().any(|one| one == "api-0"),
        "the rows stay: {texts:?}"
    );
}

#[test]
fn a_session_closed_or_deleted_lets_go_of_the_watchers_its_list_held() {
    let (mut app, ui) = listing();
    let release = Command::Cluster(cluster::Command::Release {
        reader: "resources".into(),
    });
    app.session.selected = Some(SessionId::new("b"));
    assert_eq!(
        clustered(&app, &ui),
        std::slice::from_ref(&release),
        "another session holds no cluster"
    );
    app.session
        .open
        .retain(|one| one.session.id.as_str() != "a");
    app.session.selected = None;
    assert_eq!(clustered(&app, &ui), [release]);
}
