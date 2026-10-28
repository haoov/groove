//! The kinds in the sidebar: grouped, walked, and the one listed; and the kind palette.

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
fn a_heading_folds_its_own_kinds() {
    let (app, mut ui) = listing();
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
    let texts = crate::tests::settings::drawn_whole(&app, &ui);
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

/// The kind palette's rows, as alt+k opens it and `query` narrows it.
fn picked(app: &AppState, ui: &mut Ui, query: &str) -> Vec<String> {
    let alt = crate::input::Modifiers {
        alt: true,
        ..Default::default()
    };
    press(crate::input::Key::Char('k'), alt, ui, app);
    typed(query, ui, app);
    let palette = ui.palette().expect("the kind palette");
    let rows = palette.options(app).into_iter().map(|(label, _)| label);
    rows.collect()
}

#[test]
fn alt_k_picks_a_kind_by_its_name_its_short_name_or_its_group_and_lists_it() {
    let (mut app, mut ui) = listing();
    let mut kinds = app.cluster.store.kinds("staging").expect("known").to_vec();
    kinds.push(kind("networking.k8s.io", "Ingress", "ingresses", true));
    kinds.push(KubeKind {
        short_names: vec!["po".into()],
        ..pods()
    });
    kinds.retain(|one| one.kind != "Pod" || !one.short_names.is_empty());
    kinds.push(kind("", "PodTemplate", "podtemplates", true));
    app.cluster.store.set_kinds("staging", kinds);
    ui.focus = crate::Focus::Agent;
    let rows = picked(&app, &mut ui, "po");
    assert_eq!(
        rows.first().map(String::as_str),
        Some("Pod (po)"),
        "{rows:?}"
    );
    ui.overlay = None;
    let rows = picked(&app, &mut ui, "networking");
    assert_eq!(rows, ["Ingress \u{b7} networking.k8s.io"]);
    press(crate::input::Key::Enter, Default::default(), &mut ui, &app);
    let listed = ui
        .session
        .resources
        .kind
        .as_ref()
        .map(|one| one.kind.as_str());
    assert_eq!(listed, Some("Ingress"));
    assert!(ui.palette().is_none(), "the palette shuts");
    assert_eq!(
        (ui.session.tab, ui.focus),
        (Tab::Resources, crate::Focus::Workspace),
        "the list takes the keys"
    );
}

#[test]
fn in_the_sidebar_the_arrows_step_through_the_lines_enter_folds_a_heading_or_lists_a_kind() {
    use crate::views::session::resources::{Line, shown};
    let (app, mut ui) = listing();
    ui.focus = crate::Focus::Sidebar;
    let key = |key, ui: &mut Ui| {
        press(key, Default::default(), ui, &app);
    };
    let lines = shown(&app, &ui);
    let stood = |ui: &Ui| ui.session.resources.cursor.map(|at| lines[at].clone());
    key(crate::input::Key::Down, &mut ui);
    let Some(Line::Heading(heading, _)) = stood(&ui) else {
        panic!("a heading first: {:?}", stood(&ui));
    };
    key(crate::input::Key::Enter, &mut ui);
    assert!(ui.session.resources.folded.contains(&heading));
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
    assert_eq!(ui.focus, crate::Focus::Workspace);
}

#[test]
fn the_kind_listed_stands_on_a_rounded_held_ground() {
    use groove_ui_kit::base::style::Styles;
    let (app, ui) = listing();
    let (frame, _) = crate::view(&app, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let held = Styles::new(app.config.theme(), window().tokens()).glass_chosen();
    let sm = window().tokens().sm;
    let quads = frame.layers().iter().flat_map(|layer| layer.quads.iter());
    assert!(
        quads
            .into_iter()
            .any(|one| one.color == held && one.shape.radius == sm),
        "a quad on the rounded held ground"
    );
}

#[test]
fn the_kind_the_keys_stand_on_is_lit_as_a_row_under_the_pointer_is() {
    use crate::views::session::resources::{Line, shown};
    use groove_ui_kit::base::style::Styles;
    let (app, mut ui) = listing();
    let lines = shown(&app, &ui);
    let at = lines.iter().position(|one| matches!(one, Line::Kind(_)));
    let Some(Line::Kind(stood)) = at.map(|at| lines[at].clone()) else {
        panic!("a kind listed");
    };
    ui.session.resources.cursor = at;
    let (frame, hits) = crate::view(&app, &ui, window(), &mut groove_gfx::Fonts::embedded());
    let row = hits.rect_of(&Target::ResourceKind(stood)).expect("its row");
    let lit = Styles::new(app.config.theme(), window().tokens()).glass_hover();
    let mut quads = frame.layers().iter().flat_map(|layer| layer.quads.iter());
    assert!(quads.any(|one| row.contains(one.rect.x, one.rect.y + 1.0) && one.color == lit));
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
    ui.focus = crate::Focus::Sidebar;
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
