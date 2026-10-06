//! Settings › Clusters: the contexts Groove knows as a table; only its toggles and buttons act.

use groove_controllers::cluster_service::{Event, apply};
use groove_controllers::{AppState, Command, config};
use groove_types::{ClusterChange, KubeAuth, KubeContext, Login};

use super::settings::{drawn, opened};
use super::*;
use crate::hit::Target;
use crate::views::settings::Section;

fn found(name: &str) -> KubeContext {
    KubeContext {
        name: name.into(),
        cluster: name.into(),
        server: Some(format!("https://{name}.example:6443")),
        namespace: None,
        auth: KubeAuth::Token,
    }
}

/// `hub` added and signed in, `kind` only in the kubeconfig; the section up.
fn clusters() -> (AppState, Ui) {
    let (mut app, mut ui) = opened();
    let config = r#"{ "git": { "worktree_root": "~/code" } }"#;
    app.config.config = Some(serde_json::from_str(config).expect("a config"));
    app.cluster.found = Some(vec![found("hub"), found("kind")]);
    app.config.add_cluster("hub");
    let login = Login::SignedIn {
        version: "v1.33.4".into(),
    };
    apply(
        &mut app.cluster,
        Event::Checked {
            context: "hub".into(),
            login,
        },
    );
    ui.settings.section = Section::Clusters;
    (app, ui)
}

#[test]
fn an_added_context_shows_its_settings_and_login_and_the_rest_offer_an_add() {
    let (app, mut ui) = clusters();
    let (texts, hits) = drawn(&app, &ui);
    for shown in [
        "hub",
        "signed in · v1.33.4",
        "kind",
        "https://kind.example:6443",
    ] {
        assert!(texts.iter().any(|one| one == shown), "{shown}: {texts:?}");
    }
    let add = hits
        .rect_of(&Target::ClusterAdd("kind".into()))
        .expect("kind's add");
    let asked = click(add, &mut ui, &app, &hits);
    let added = config::Command::AddCluster {
        context: "kind".into(),
    };
    assert_eq!(asked, [Command::Config(added)]);
}

#[test]
fn each_line_lights_under_the_pointer_and_only_its_toggles_and_buttons_act() {
    let (app, mut ui) = clusters();
    let (_, hits) = drawn(&app, &ui);
    assert!(hits.rect_of(&Target::ClusterRow("hub".into())).is_some());
    assert!(hits.rect_of(&Target::ClusterRow("kind".into())).is_some());
    let on = ClusterChange::ReadOnly(true);
    let toggle = hits.rect_of(&Target::ClusterSet("hub".into(), on.clone()));
    let asked = click(toggle.expect("the read-only toggle"), &mut ui, &app, &hits);
    let set = config::Command::SetCluster {
        context: "hub".into(),
        change: on,
    };
    assert_eq!(asked, [Command::Config(set)]);
    let row = hits
        .rect_of(&Target::ClusterRow("hub".into()))
        .expect("hub's line");
    let name = groove_gfx::Rect::new(row.x, row.y, row.h * 2.0, row.h);
    assert_eq!(
        hits.at(name.x + 1.0, name.y + 1.0),
        Some(Target::ClusterRow("hub".into()))
    );
    assert!(click(name, &mut ui, &app, &hits).is_empty());
}
