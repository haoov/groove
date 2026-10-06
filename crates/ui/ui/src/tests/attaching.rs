//! A session's clusters: attached and detached from the palette, named on the overview.

use groove_controllers::session::Command as Session;
use groove_controllers::{AppState, Command};
use groove_types::{Attached, SessionId};

use crate::input::Key;
use crate::palette::Palette;
use crate::tests::{full_app, metrics};
use crate::{Ui, view};

fn on(context: &str, namespace: Option<&str>) -> Attached {
    Attached {
        context: context.into(),
        namespace: namespace.map(str::to_string),
    }
}

/// `hub` and `staging` added to Groove; the session holds staging on `paxone` and hub whole.
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
    open.clusters = vec![on("staging", Some("paxone")), on("hub", None)];
    app
}

fn typed(palette: &mut Palette, text: &str, app: &AppState) {
    for c in text.chars() {
        palette.key(Key::Char(c), app);
    }
}

fn asked(app: &AppState, flow: &str, answers: &[&str]) -> Command {
    let mut palette = Palette::default();
    typed(&mut palette, flow, app);
    palette.key(Key::Enter, app);
    for answer in answers {
        typed(&mut palette, answer, app);
        let done = palette.key(Key::Enter, app);
        if let Some(command) = done.commands.into_iter().next() {
            return command;
        }
    }
    panic!("{flow} asked for more than {answers:?}")
}

#[test]
fn attach_picks_a_context_groove_knows_then_a_namespace_or_none_for_the_whole_cluster() {
    let app = attaching();
    let session = SessionId::new("a");
    let whole = Command::Session(Session::AttachCluster {
        session: session.clone(),
        attached: on("hub", None),
    });
    assert_eq!(asked(&app, "attach cluster", &["hub", ""]), whole);
    let named = Session::AttachCluster {
        session,
        attached: on("staging", Some("cnpg")),
    };
    assert_eq!(
        asked(&app, "attach cluster", &["staging", "cnpg"]),
        Command::Session(named)
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
    let (frame, _) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let texts: Vec<String> = frame.layers()[0]
        .texts
        .iter()
        .map(|one| one.text.clone())
        .collect();
    for shown in ["CLUSTERS", "staging", "paxone", "hub", "whole cluster"] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    assert_eq!(texts.iter().filter(|one| *one == "staging").count(), 1);
}
