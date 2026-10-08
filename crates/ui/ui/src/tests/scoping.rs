//! The scope pickers' panel: what the session holds, checked or not, then what it could attach.

use groove_controllers::session::Command as Session;
use groove_controllers::{AppState, Command, cluster};
use groove_types::{Attached, SessionId};

use crate::hit::{Hits, Picks, Target};
use crate::input::Key;
use crate::tests::settings::drawn;
use crate::tests::{CTRL_SHIFT, click, full_app, press};
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
    let names = ["paxone", "cnpg", "monitoring"].map(String::from).to_vec();
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
fn the_scope_line_stands_on_every_tab_and_attaches_a_first_context_and_namespace() {
    let app = holding(Vec::new());
    let mut ui = Ui::default();
    let (texts, hits) = drawn(&app, &ui);
    assert!(texts.iter().any(|one| one == "no cluster"), "{texts:?}");
    assert!(hits.rect_of(&Target::Picker(Picks::Namespaces)).is_none());
    picker(&app, &mut ui, Picks::Contexts);
    assert_eq!(labels(&app, &ui), ["available", "hub", "staging"]);
    let listed = chose(&app, &mut ui, "staging", false);
    let list = cluster::Command::ListNamespaces {
        context: "staging".into(),
    };
    assert_eq!(listed, [Command::Cluster(list)]);
    assert_eq!(
        labels(&app, &ui),
        ["staging", "* whole cluster", "paxone", "cnpg", "monitoring"]
    );
    let attached = chose(&app, &mut ui, "cnpg", false);
    let attach = Session::AttachCluster {
        session: SessionId::new("a"),
        attached: on("staging", Some("cnpg")),
    };
    assert_eq!(attached, [session(attach)]);
    assert!(
        matches!(ui.overlay, Some(Overlay::Scope(_))),
        "the panel stays"
    );
}

#[test]
fn the_namespace_panel_lists_the_held_first_checks_them_detaches_them_and_attaches_the_rest() {
    let app = holding(vec![
        on("staging", Some("paxone")),
        on("staging", Some("cnpg")),
    ]);
    let mut ui = Ui::default();
    let (opened, _) = picker(&app, &mut ui, Picks::Namespaces);
    let list = cluster::Command::ListNamespaces {
        context: "staging".into(),
    };
    assert_eq!(opened, [Command::Cluster(list)]);
    assert_eq!(
        labels(&app, &ui),
        ["staging", "paxone", "cnpg", "* whole cluster", "monitoring"]
    );
    assert!(chose(&app, &mut ui, "paxone", false).is_empty());
    assert!(
        ui.session
            .resources
            .hidden
            .contains(&on("staging", Some("paxone")))
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
    for c in "dev-x".chars() {
        press(Key::Char(c), Default::default(), &mut ui, &app);
    }
    assert_eq!(labels(&app, &ui), ["staging", "attach dev-x"]);
    press(Key::Escape, Default::default(), &mut ui, &app);
    assert!(ui.overlay.is_none());
}

#[test]
fn ctrl_shift_n_then_a_name_and_enter_shows_that_namespace_alone_on_the_resources_tab() {
    let app = holding(vec![
        on("staging", Some("paxone")),
        on("staging", Some("cnpg")),
    ]);
    let mut ui = Ui {
        focus: crate::Focus::Workspace,
        ..Ui::default()
    };
    press(Key::Char('n'), CTRL_SHIFT, &mut ui, &app);
    for c in "cnp".chars() {
        press(Key::Char(c), Default::default(), &mut ui, &app);
    }
    press(Key::Enter, Default::default(), &mut ui, &app);
    assert!(ui.overlay.is_none(), "shown alone, the panel shuts");
    assert_eq!(ui.session.tab, crate::views::session::Tab::Resources);
    let hidden: Vec<Attached> = ui.session.resources.hidden.iter().cloned().collect();
    assert_eq!(hidden, [on("staging", Some("paxone"))]);
    press(Key::Char('n'), CTRL_SHIFT, &mut ui, &app);
    press(Key::Down, Default::default(), &mut ui, &app);
    press(Key::Char(' '), Default::default(), &mut ui, &app);
    assert!(
        ui.session.resources.hidden.is_empty(),
        "space checks paxone again"
    );
}

#[test]
fn under_a_whole_cluster_a_namespace_is_offered_and_its_small_check_is_green_its_cross_red() {
    use groove_ui_kit::base::style::{Role, Styles};
    let app = holding(vec![on("staging", None)]);
    let mut ui = Ui::default();
    picker(&app, &mut ui, Picks::Namespaces);
    assert_eq!(
        labels(&app, &ui),
        ["staging", "* whole cluster", "paxone", "cnpg", "monitoring"]
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
