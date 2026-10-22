//! The scope pickers' panel: what the session holds, checked or not, then what it could attach.

use groove_controllers::session::Command as Session;
use groove_controllers::{AppState, Command, cluster};
use groove_types::{Attached, SessionId};

use crate::hit::{Hits, Picks, Target};
use crate::input::Key;
use crate::tests::drawn;
use crate::tests::{CTRL_SHIFT, click, full_app, press, typed};
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
    let lines = scope_lines(app, open, &ui.session.resources, scoping);
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

fn picker(app: &AppState, ui: &mut Ui, which: Picks) -> (Vec<Command>, Hits) {
    let (_, hits) = drawn(app, ui);
    let rect = hits.rect_of(&Target::Picker(which)).expect("the picker");
    (click(rect, ui, app, &hits), hits)
}

fn session(command: Session) -> Command {
    Command::Session(command)
}

#[test]
fn the_scope_line_stands_on_the_kubernetes_tab_alone() {
    let bare = holding(Vec::new());
    let (texts, hits) = drawn(&bare, &Ui::default());
    assert!(!texts.iter().any(|one| one == "no cluster"), "{texts:?}");
    assert!(hits.rect_of(&Target::Picker(Picks::Contexts)).is_none());
    let app = holding(vec![on("staging", Some("paxone"))]);
    let (_, hits) = drawn(&app, &Ui::default());
    assert!(
        hits.rect_of(&Target::Picker(Picks::Contexts)).is_none(),
        "not on the overview"
    );
    let mut ui = on_kubernetes();
    let (opened, _) = picker(&app, &mut ui, Picks::Namespaces);
    let list = cluster::Command::ListNamespaces {
        context: "staging".into(),
    };
    assert_eq!(opened, [Command::Cluster(list)]);
    assert!(matches!(ui.overlay, Some(Overlay::Scope(_))));
}

fn on_kubernetes() -> Ui {
    let mut ui = Ui::default();
    ui.session.tab = crate::views::session::Tab::Resources;
    ui
}

#[test]
fn the_namespace_panel_lists_the_held_first_checks_them_detaches_them_and_attaches_the_rest() {
    let app = holding(vec![
        on("staging", Some("shop")),
        on("staging", Some("cnpg")),
    ]);
    let mut ui = on_kubernetes();
    let (opened, _) = picker(&app, &mut ui, Picks::Namespaces);
    let list = cluster::Command::ListNamespaces {
        context: "staging".into(),
    };
    assert_eq!(opened, [Command::Cluster(list)]);
    assert_eq!(
        labels(&app, &ui),
        ["staging", "shop", "cnpg", "* whole cluster", "monitoring"]
    );
    assert!(chose(&app, &mut ui, "shop", false).is_empty());
    assert!(
        ui.session
            .resources
            .hidden
            .contains(&on("staging", Some("shop")))
    );
    assert!(
        matches!(ui.overlay, Some(Overlay::Scope(_))),
        "a check keeps the panel"
    );
    let detach = Session::DetachCluster {
        session: SessionId::new("a"),
        attached: on("staging", Some("cnpg")),
    };
    assert_eq!(chose(&app, &mut ui, "cnpg", true), [session(detach)]);
    let attach = Session::AttachCluster {
        session: SessionId::new("a"),
        attached: on("staging", Some("monitoring")),
    };
    assert_eq!(chose(&app, &mut ui, "monitoring", false), [session(attach)]);
    typed("dev-x", &mut ui, &app);
    assert_eq!(labels(&app, &ui), ["staging", "attach dev-x"]);
    press(Key::Escape, Default::default(), &mut ui, &app);
    assert!(ui.overlay.is_none());
}

#[test]
fn ctrl_shift_n_then_a_name_and_enter_shows_that_namespace_alone_on_the_resources_tab() {
    let app = holding(vec![
        on("staging", Some("shop")),
        on("staging", Some("cnpg")),
    ]);
    let mut ui = Ui {
        focus: crate::Focus::Workspace,
        ..Ui::default()
    };
    press(Key::Char('n'), CTRL_SHIFT, &mut ui, &app);
    typed("cnp", &mut ui, &app);
    press(Key::Enter, Default::default(), &mut ui, &app);
    assert!(ui.overlay.is_none(), "shown alone, the panel shuts");
    assert_eq!(ui.session.tab, crate::views::session::Tab::Resources);
    let hidden: Vec<Attached> = ui.session.resources.hidden.iter().cloned().collect();
    assert_eq!(hidden, [on("staging", Some("shop"))]);
    press(Key::Char('n'), CTRL_SHIFT, &mut ui, &app);
    press(Key::Down, Default::default(), &mut ui, &app);
    press(Key::Char(' '), Default::default(), &mut ui, &app);
    assert!(
        ui.session.resources.hidden.is_empty(),
        "space checks shop again"
    );
}

#[test]
fn enter_on_a_namespace_not_held_attaches_it_alone() {
    let app = holding(vec![
        on("staging", Some("shop")),
        on("staging", Some("cnpg")),
    ]);
    let mut ui = Ui {
        focus: crate::Focus::Workspace,
        ..Ui::default()
    };
    press(Key::Char('n'), CTRL_SHIFT, &mut ui, &app);
    typed("monit", &mut ui, &app);
    let attach = Session::AttachCluster {
        session: SessionId::new("a"),
        attached: on("staging", Some("monitoring")),
    };
    let sent = press(Key::Enter, Default::default(), &mut ui, &app);
    assert_eq!(sent, [session(attach)]);
    assert!(ui.overlay.is_none(), "shown alone, the panel shuts");
    let hidden: Vec<Attached> = ui.session.resources.hidden.iter().cloned().collect();
    assert_eq!(
        hidden,
        [on("staging", Some("cnpg")), on("staging", Some("shop"))]
    );
}

#[test]
fn under_a_whole_cluster_a_namespace_is_offered_and_its_small_check_is_green_its_cross_red() {
    use groove_ui_kit::base::style::{Role, Styles};
    let app = holding(vec![on("staging", None)]);
    let mut ui = on_kubernetes();
    picker(&app, &mut ui, Picks::Namespaces);
    assert_eq!(
        labels(&app, &ui),
        ["staging", "* whole cluster", "shop", "cnpg", "monitoring"]
    );
    let attach = Session::AttachCluster {
        session: SessionId::new("a"),
        attached: on("staging", Some("cnpg")),
    };
    assert_eq!(chose(&app, &mut ui, "cnpg", false), [session(attach)]);
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
    let app = holding(vec![
        on("staging", Some("shop")),
        on("staging", Some("cnpg")),
    ]);
    let mut ui = Ui {
        focus: crate::Focus::Workspace,
        ..Ui::default()
    };
    press(Key::Char('n'), CTRL_SHIFT, &mut ui, &app);
    let asked = crate::input::handle(
        crate::input::Input::Paste("cnp".into()),
        &mut ui,
        &app,
        &Hits::default(),
        crate::tests::window(),
    );
    assert!(asked.is_empty(), "nothing else takes it: {asked:?}");
    press(Key::Enter, Default::default(), &mut ui, &app);
    let hidden: Vec<Attached> = ui.session.resources.hidden.iter().cloned().collect();
    assert_eq!(hidden, [on("staging", Some("shop"))]);
}
