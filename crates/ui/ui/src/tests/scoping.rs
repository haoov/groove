//! The scope panel: the contexts held and to attach, or every `context/namespace` ranked.

use groove_controllers::session::Command as Session;
use groove_controllers::{AppState, Command, cluster};
use groove_types::{Attached, SessionId};

use crate::hit::{Hits, Picks, Target};
use crate::input::Key;
use crate::tests::drawn;
use crate::tests::{ALT, click, full_app, press, typed};
use crate::views::session::resources::{ScopeLine, scope_lines};
use crate::{Overlay, Ui};

fn on(context: &str, namespace: Option<&str>) -> Attached {
    Attached {
        context: context.into(),
        namespace: namespace.map(str::to_string),
    }
}

/// `hub` and `staging` added to Groove; the session holds `pairs`; staging lists three namespaces.
fn holding(pairs: Vec<Attached>) -> AppState {
    let mut app = full_app();
    let config = r#"{ "git": { "worktree_root": "~/code" } }"#;
    app.config.config = Some(serde_json::from_str(config).expect("a config"));
    app.config.add_cluster("hub");
    app.config.add_cluster("staging");
    let names = ["shop", "cnpg", "monitoring"].map(String::from).to_vec();
    app.cluster.store.set_namespaces("staging", names);
    let open = app
        .session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session");
    open.clusters = pairs;
    app
}

fn labels(app: &AppState, ui: &Ui) -> Vec<String> {
    let (Some(open), Some(Overlay::Scope(scoping))) = (app.session.selected(), &ui.overlay) else {
        return Vec::new();
    };
    let lines = scope_lines(app, open, scoping);
    lines.iter().map(ScopeLine::label).collect()
}

/// A click on the panel's line that reads `label`, or on its × with `detach`.
fn chose(app: &AppState, ui: &mut Ui, label: &str, detach: bool) -> Vec<Command> {
    let at = labels(app, ui).iter().position(|one| one == label);
    let at = at.unwrap_or_else(|| panic!("{label} in {:?}", labels(app, ui)));
    let (_, hits) = drawn(app, ui);
    let target = match detach {
        true => Target::ScopeDetach(at),
        false => Target::ScopeLine(at),
    };
    let rect = hits.rect_of(&target).expect("the line drawn");
    click(rect, ui, app, &hits)
}

/// The panel `alt+n` or `alt+c` opens.
fn picker(app: &AppState, ui: &mut Ui, which: Picks) -> Vec<Command> {
    let key = match which {
        Picks::Contexts => 'c',
        _ => 'n',
    };
    press(Key::Char(key), ALT, ui, app)
}

fn session(command: Session) -> Command {
    Command::Session(command)
}

/// The pairs the list reads.
fn held(app: &AppState, ui: &Ui) -> Vec<String> {
    let open = app.session.selected().expect("the session");
    let pairs = crate::views::session::resources::held(open, &ui.session.resources);
    let label = |pair: &Attached| ScopeLine::Namespace(pair.clone()).label();
    pairs.iter().map(label).collect()
}

#[test]
fn the_scope_line_only_tells_and_alt_n_opens_the_panel_from_any_tab() {
    let app = holding(vec![on("staging", None)]);
    let mut ui = on_kubernetes();
    let (texts, hits) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "staging"), "{texts:?}");
    for which in [Picks::Contexts, Picks::Namespaces] {
        assert!(hits.rect_of(&Target::Picker(which)).is_none(), "{which:?}");
    }
    ui.session.tab = crate::views::session::Tab::Diff;
    let opened = picker(&app, &mut ui, Picks::Namespaces);
    let list = cluster::Command::ListNamespaces {
        context: "staging".into(),
    };
    assert_eq!(opened, [Command::Cluster(list)]);
    assert!(matches!(ui.overlay, Some(Overlay::Scope(_))));
    chose(&app, &mut ui, "staging/shop", false);
    assert_eq!(ui.session.tab, crate::views::session::Tab::Resources);
}

#[test]
fn alt_c_opens_the_context_panel_and_again_shuts_it() {
    let app = holding(vec![on("staging", None)]);
    let mut ui = Ui::default();
    picker(&app, &mut ui, Picks::Contexts);
    assert_eq!(
        labels(&app, &ui),
        ["in this session", "staging", "available", "hub"]
    );
    picker(&app, &mut ui, Picks::Namespaces);
    assert_eq!(
        labels(&app, &ui).first().map(String::as_str),
        Some("staging/*")
    );
    picker(&app, &mut ui, Picks::Namespaces);
    assert!(ui.overlay.is_none());
}

fn on_kubernetes() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = crate::views::session::Tab::Resources;
    ui
}

#[test]
fn the_namespace_panel_lists_each_context_and_namespace_and_a_click_shows_one_more() {
    let app = holding(vec![on("staging", None)]);
    let mut ui = on_kubernetes();
    picker(&app, &mut ui, Picks::Namespaces);
    assert_eq!(
        labels(&app, &ui),
        [
            "staging/*",
            "staging/shop",
            "staging/cnpg",
            "staging/monitoring"
        ]
    );
    assert!(chose(&app, &mut ui, "staging/shop", false).is_empty());
    assert_eq!(
        held(&app, &ui),
        ["staging/shop"],
        "the whole cluster narrows"
    );
    assert!(
        matches!(ui.overlay, Some(Overlay::Scope(_))),
        "a check keeps the panel"
    );
    chose(&app, &mut ui, "staging/cnpg", false);
    assert_eq!(held(&app, &ui), ["staging/cnpg", "staging/shop"]);
    chose(&app, &mut ui, "staging/shop", false);
    assert_eq!(held(&app, &ui), ["staging/cnpg"]);
    chose(&app, &mut ui, "staging/*", false);
    assert_eq!(held(&app, &ui), ["staging/*"]);
    press(Key::Escape, Default::default(), &mut ui, &app);
    assert!(ui.overlay.is_none());
}

#[test]
fn alt_n_then_a_ranked_query_and_enter_shows_that_namespace_alone_and_space_adds_one() {
    let mut app = holding(vec![on("staging", None), on("hub", None)]);
    app.cluster
        .store
        .set_namespaces("hub", vec!["argocd".into(), "shop-cnpg".into()]);
    let mut ui = Ui::default();
    press(Key::Char('n'), ALT, &mut ui, &app);
    typed("st/cn", &mut ui, &app);
    assert_eq!(
        labels(&app, &ui).first().map(String::as_str),
        Some("staging/cnpg")
    );
    press(Key::Enter, Default::default(), &mut ui, &app);
    assert!(ui.overlay.is_none(), "shown alone, the panel shuts");
    assert_eq!(ui.session.tab, crate::views::session::Tab::Resources);
    assert_eq!(ui.focus, crate::Focus::Workspace, "the list takes the keys");
    assert_eq!(held(&app, &ui), ["staging/cnpg"]);
    press(Key::Char('n'), ALT, &mut ui, &app);
    typed("argo", &mut ui, &app);
    press(Key::Char(' '), Default::default(), &mut ui, &app);
    assert_eq!(held(&app, &ui), ["staging/cnpg", "hub/argocd"]);
    assert!(
        matches!(ui.overlay, Some(Overlay::Scope(_))),
        "space keeps it"
    );
}

#[test]
fn a_context_held_by_its_namespaces_offers_those_alone() {
    let app = holding(vec![
        on("staging", Some("shop")),
        on("staging", Some("cnpg")),
    ]);
    let mut ui = on_kubernetes();
    let opened = picker(&app, &mut ui, Picks::Namespaces);
    assert!(opened.is_empty(), "nothing to list");
    assert_eq!(labels(&app, &ui), ["staging/shop", "staging/cnpg"]);
}

#[test]
fn the_context_panel_attaches_a_known_context_whole_and_detaches_a_held_one() {
    let app = holding(vec![on("staging", None)]);
    let mut ui = on_kubernetes();
    picker(&app, &mut ui, Picks::Contexts);
    assert_eq!(
        labels(&app, &ui),
        ["in this session", "staging", "available", "hub"]
    );
    let attach = Session::AttachCluster {
        session: SessionId::new("a"),
        attached: on("hub", None),
    };
    assert_eq!(chose(&app, &mut ui, "hub", false), [session(attach)]);
    let detach = Session::DetachCluster {
        session: SessionId::new("a"),
        attached: on("staging", None),
    };
    assert_eq!(chose(&app, &mut ui, "staging", true), [session(detach)]);
}

#[test]
fn a_shown_line_s_small_check_is_green_and_a_held_context_s_cross_red() {
    use groove_ui_kit::base::style::{Role, Styles};
    let app = holding(vec![on("staging", None)]);
    let mut ui = on_kubernetes();
    picker(&app, &mut ui, Picks::Contexts);
    let (frame, _) = crate::view(
        &app,
        &ui,
        crate::tests::window(),
        &mut groove_gfx::Fonts::embedded(),
    );
    let styles = Styles::new(app.config.theme(), crate::tests::window().tokens());
    let icons: Vec<_> = frame
        .layers()
        .iter()
        .flat_map(|one| one.icons.iter())
        .collect();
    let tinted = |icon, role| {
        icons
            .iter()
            .any(|one| one.icon == icon && one.color == styles.color(role))
    };
    assert!(
        tinted(groove_gfx::Icon::Ticked, Role::Ok),
        "the check is green"
    );
    assert!(
        tinted(groove_gfx::Icon::Cross, Role::Bad),
        "the cross is red"
    );
    let small = crate::tests::window().tokens().small;
    let sized = |icon| {
        icons
            .iter()
            .any(|one| one.icon == icon && one.rect.w == small)
    };
    assert!(sized(groove_gfx::Icon::Ticked), "the check is small");
    assert!(sized(groove_gfx::Icon::Cross), "the cross is small");
}

#[test]
fn a_paste_goes_into_the_panel_s_search_and_enter_shows_what_it_names() {
    let app = holding(vec![on("staging", None)]);
    let mut ui = Ui::default();
    press(Key::Char('n'), ALT, &mut ui, &app);
    let asked = crate::input::handle(
        crate::input::Input::Paste("cnp".into()),
        &mut ui,
        &app,
        &Hits::default(),
        crate::tests::window(),
    );
    assert!(asked.is_empty(), "nothing else takes it: {asked:?}");
    press(Key::Enter, Default::default(), &mut ui, &app);
    assert_eq!(held(&app, &ui), ["staging/cnpg"]);
}

#[test]
fn the_panel_stands_on_glass() {
    let app = holding(vec![on("staging", None)]);
    let mut ui = on_kubernetes();
    let panes = |ui: &Ui| {
        let at = crate::tests::metrics(1600, 1000, 1.0);
        let (frame, _) = crate::view(&app, ui, at, &mut groove_gfx::Fonts::embedded());
        frame
            .layers()
            .iter()
            .map(|layer| layer.glass.len())
            .sum::<usize>()
    };
    let shut = panes(&ui);
    picker(&app, &mut ui, Picks::Namespaces);
    assert_eq!(panes(&ui), shut + 1);
}
