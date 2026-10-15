//! The kinds in the sidebar: grouped, searched, and the one listed.

use super::*;

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

#[test]
fn a_click_on_the_sidebar_s_bar_and_typing_narrow_the_kinds_and_leave_the_rows_be() {
    let (app, mut ui) = listing();
    ui.focus = crate::Focus::Agent;
    let (_, hits) = drawn(&app, &ui);
    let bar = hits
        .rect_of(&Target::ResourceFilter)
        .expect("the sidebar's bar");
    click(bar, &mut ui, &app, &hits);
    typed("node", &mut ui, &app);
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
fn the_kind_the_keys_stand_on_is_raised_as_a_selected_row_is() {
    use crate::views::session::resources::{Line, shown};
    use groove_ui_kit::base::style::Styles;
    let (app, mut ui) = listing();
    let lines = shown(&app, &ui);
    let at = lines.iter().position(|one| matches!(one, Line::Kind(_)));
    let Some(Line::Kind(stood)) = at.map(|at| lines[at].clone()) else {
        panic!("a kind listed");
    };
    (ui.session.resources.filtering, ui.session.resources.cursor) = (true, at);
    let (frame, hits) = crate::view(&app, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let row = hits.rect_of(&Target::ResourceKind(stood)).expect("its row");
    let raised = Styles::new(app.config.theme(), window().tokens()).raised();
    let mut quads = frame.layers().iter().flat_map(|layer| layer.quads.iter());
    assert!(quads.any(|one| one.rect == row && one.color == raised));
}

#[test]
fn a_paste_in_the_kind_search_stands_on_the_first_kind_as_typing_does() {
    use crate::views::session::resources::{Line, shown};
    let (app, mut ui) = listing();
    (ui.session.resources.filtering, ui.session.resources.cursor) = (true, Some(0));
    crate::input::handle(
        crate::input::Input::Paste("dep".into()),
        &mut ui,
        &app,
        &crate::hit::Hits::default(),
        window(),
    );
    let lines = shown(&app, &ui);
    let first = lines.iter().position(|one| matches!(one, Line::Kind(_)));
    assert_eq!(ui.session.resources.cursor, first);
    assert_ne!(first, Some(0), "a heading stands first");
}

#[test]
fn the_arrows_scroll_the_kinds_to_keep_the_line_stood_on_in_sight() {
    use crate::hit::Scroller;
    let (mut app, mut ui) = listing();
    let many = (0..80).map(|at| {
        let name = format!("Thing{at}");
        kind("example.io", &name, &format!("things{at}"), true)
    });
    app.cluster.store.set_kinds("staging", many.collect());
    ui.session.resources.filtering = true;
    let lines = crate::views::session::resources::shown(&app, &ui).len();
    let (_, hits) = drawn(&app, &ui);
    let extent = hits.extent(Scroller::Kinds);
    assert!(extent > 0.0, "the kinds run past the sidebar");
    for _ in 0..lines {
        let (_, hits) = drawn(&app, &ui);
        let input = crate::input::Input::Key {
            key: crate::input::Key::Down,
            mods: Default::default(),
        };
        crate::input::handle(input, &mut ui, &app, &hits, window());
    }
    assert_eq!(ui.session.resources.cursor, Some(lines - 1));
    assert_eq!(ui.session.resources.kinds, extent, "the last line in sight");
}

#[test]
fn the_kind_the_search_names_comes_before_scattered_letters() {
    let (mut app, mut ui) = listing();
    let hpa = kind(
        "autoscaling",
        "HorizontalPodAutoscaler",
        "horizontalpodautoscalers",
        true,
    );
    let kinds = vec![pods(), hpa, kind("", "Node", "nodes", false)];
    app.cluster.store.set_kinds("staging", kinds);
    ui.session.resources.filter.set("nodes");
    let (texts, _) = drawn(&app, &ui);
    let at = |label: &str| texts.iter().position(|one| one == label);
    let nodes = at("Nodes").expect("nodes");
    let hpa = texts.iter().position(|one| one.starts_with("Horizontal"));
    assert!(
        hpa.is_none_or(|hpa| nodes < hpa),
        "the best match first: {texts:?}"
    );
}
