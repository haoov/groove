//! A session's clusters: attached and detached from the palette, named on the overview.

use groove_controllers::session::Command as Session;
use groove_controllers::{AppState, Command};
use groove_types::{Attached, SessionId};

use crate::Ui;
use crate::input::Key;
use crate::palette::Palette;
use crate::tests::{full_app, metrics, render, typed_into};

fn on(context: &str, namespace: Option<&str>) -> Attached {
    Attached {
        context: context.into(),
        namespace: namespace.map(str::to_string),
    }
}

/// `hub` and `staging` added to Groove; the session holds staging on `shop` and hub whole.
fn attaching() -> AppState {
    let mut app = full_app();
    let config = r#"{ "git": { "worktree_root": "~/code" } }"#;
    app.config.config = Some(serde_json::from_str(config).expect("a config"));
    app.config.add_cluster("hub");
    app.config.add_cluster("staging");
    let open = app
        .session
        .get_mut(&SessionId::new("a"))
        .expect("the fixture's session");
    open.clusters = vec![on("staging", Some("shop")), on("hub", None)];
    app
}

fn asked(app: &AppState, flow: &str, answers: &[&str]) -> Command {
    let mut palette = Palette::default();
    typed_into(&mut palette, flow, app);
    palette.key(Key::Enter, app);
    for answer in answers {
        typed_into(&mut palette, answer, app);
        let done = palette.key(Key::Enter, app);
        let session = done
            .commands
            .into_iter()
            .find(|one| matches!(one, Command::Session(_)));
        if let Some(command) = session {
            return command;
        }
    }
    panic!("{flow} asked for more than {answers:?}")
}

#[test]
fn attach_picks_a_context_groove_knows_and_holds_its_whole_cluster() {
    let app = attaching();
    let whole = Session::AttachCluster {
        session: SessionId::new("a"),
        attached: on("staging", None),
    };
    assert_eq!(
        asked(&app, "attach cluster", &["staging"]),
        Command::Session(whole)
    );
}

#[test]
fn detach_picks_one_context_and_namespace_the_session_holds() {
    let app = attaching();
    let detach = Session::DetachCluster {
        session: SessionId::new("a"),
        attached: on("hub", None),
    };
    assert_eq!(
        asked(&app, "detach cluster", &["hub"]),
        Command::Session(detach)
    );
}

#[test]
fn the_overview_names_each_context_once_with_its_namespaces_under_it() {
    let app = attaching();
    let (texts, _) = render(&app, &Ui::default(), metrics(1280, 800, 1.0));
    for shown in ["CLUSTERS", "staging", "shop", "hub", "whole cluster"] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    assert_eq!(texts.iter().filter(|one| *one == "staging").count(), 1);
}

#[test]
fn alt_shift_4_opens_the_resources_tab_while_the_session_holds_a_cluster() {
    let alt_shift = crate::input::Modifiers {
        alt: true,
        shift: true,
        ..Default::default()
    };
    let mut ui = Ui::default();
    crate::tests::press(Key::Char('4'), alt_shift, &mut ui, &full_app());
    assert_eq!(
        ui.session.tab,
        crate::views::session::Tab::Overview,
        "no cluster, no tab"
    );
    crate::tests::press(Key::Char('4'), alt_shift, &mut ui, &attaching());
    assert_eq!(ui.session.tab, crate::views::session::Tab::Resources);
}
