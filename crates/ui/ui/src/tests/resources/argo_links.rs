//! An Application's links: a resource opens on its destination's context, a source's repo and commit in the session.

use groove_controllers::{AppState, Command, cluster, workspace};
use groove_types::{KubeAuth, KubeContext};

use super::argo::failing;
use super::argo_sync::{hit, managed, opened_application};
use super::kind;
use crate::Ui;
use crate::hit::Target;
use crate::views::session::Tab;
use crate::views::session::resources::{Link, Opens, View};

const SHA: &str = "e9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c3d2e1f0";
const REPO: &str = "https://gitlab.example.com/billing-eu-deploy.git";

fn asked(app: &AppState, ui: &Ui) -> Vec<Command> {
    crate::frame_commands(app, ui, crate::tests::metrics(1920, 1080, 1.0))
}

#[test]
fn a_resource_opens_on_the_context_whose_server_the_destination_names() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    let mut api = managed("Deployment", "billing-api", "OutOfSync", 0);
    api.group = "apps".into();
    part.resources = vec![api];
    let (mut app, mut ui) = opened_application(object);
    let read = Command::Cluster(cluster::Command::ReadContexts);
    assert!(asked(&app, &ui).contains(&read), "the servers read first");

    app.config.add_cluster("prod");
    app.cluster.found = Some(vec![KubeContext {
        name: "prod".into(),
        cluster: "prod".into(),
        server: Some("https://10.0.0.1:6443/".into()),
        namespace: None,
        auth: KubeAuth::None,
    }]);
    let discover = Command::Cluster(cluster::Command::Discover {
        context: "prod".into(),
        again: false,
    });
    assert!(asked(&app, &ui).contains(&discover), "then its kinds");

    let deployments = kind("apps", "Deployment", "deployments", true);
    app.cluster
        .store
        .set_kinds("prod", vec![deployments.clone()]);
    let link = Link {
        context: "prod".into(),
        kind: deployments,
        namespace: Some("billing-acc".into()),
        name: "billing-api".into(),
    };
    hit(&app, &mut ui, Target::ResourceOpen(Box::new(link.clone())));
    let shown = ui.session.resources.tab().map(|one| &one.link);
    assert_eq!(shown, Some(&link));
}

#[test]
fn a_destination_matches_its_server_written_another_way() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.destination.server = Some("https://Kube.Example.com:443".into());
    let (mut app, ui) = opened_application(object);
    app.config.add_cluster("prod");
    app.cluster.found = Some(vec![KubeContext {
        name: "prod".into(),
        cluster: "prod".into(),
        server: Some("https://kube.example.com/".into()),
        namespace: None,
        auth: KubeAuth::None,
    }]);
    let discover = Command::Cluster(cluster::Command::Discover {
        context: "prod".into(),
        again: false,
    });
    assert!(asked(&app, &ui).contains(&discover));
}

/// A ctrl+click on the source's value `text` that opens `opens`.
fn followed(app: &AppState, ui: &mut Ui, text: &str, opens: Opens) -> Vec<Command> {
    let metrics = crate::tests::metrics(1920, 1080, 1.0);
    let (_, hits) = crate::view(app, ui, metrics, &mut groove_gfx::Fonts::embedded());
    let target = Target::ResourceCopy(text.into(), Some(Box::new(opens)));
    let at = hits
        .rect_of(&target)
        .unwrap_or_else(|| panic!("{text} links"));
    let ctrl = crate::input::Modifiers {
        ctrl: true,
        ..Default::default()
    };
    let press = crate::input::Input::Press {
        x: at.x + 1.0,
        y: at.y + at.h / 2.0,
        mods: ctrl,
    };
    crate::input::handle(press, ui, app, &hits, metrics)
}

#[test]
fn a_ctrl_click_on_a_git_source_shows_its_repo_and_on_its_revision_that_commit() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.sources[0].synced = Some(SHA.into());
    let (mut app, mut ui) = opened_application(object);
    hit(&app, &mut ui, Target::ResourceView(View::Describe));
    ui.settle(&app);

    let repo = Opens::Repo {
        repo: REPO.into(),
        target: "main".into(),
    };
    let sent = followed(&app, &mut ui, "gitlab.example.com/billing-eu-deploy", repo);
    let show = |sha: Option<&str>| {
        Command::Workspace(workspace::Command::ShowRepo {
            repo: REPO.into(),
            target: Some("main".into()),
            sha: sha.map(String::from),
        })
    };
    assert_eq!(sent, [show(None)]);
    let (diff, files, resources) = (Tab::Diff, Tab::Files, Tab::Resources);
    app.workspace.revision = Some(groove_controllers::workspace_service::Revision {
        session: app.session.selected.clone().expect("a session"),
        slug: "gitlab.example.com/billing-eu-deploy".into(),
        sha: None,
        reading: false,
    });
    ui.settle(&app);
    assert_eq!(
        ui.session.tab, resources,
        "the Kubernetes tab until the repo is selected"
    );
    app.workspace.revision = None;
    ui.settle(&app);
    assert_eq!(ui.session.tab, files, "then its files");

    ui.session.tab = resources;
    let revision = Opens::Revision {
        repo: REPO.into(),
        target: "main".into(),
        sha: SHA.into(),
    };
    let sent = followed(&app, &mut ui, SHA, revision);
    assert_eq!(sent, [show(Some(SHA))]);
    app.workspace.revision = Some(groove_controllers::workspace_service::Revision {
        session: app.session.selected.clone().expect("a session"),
        slug: "gitlab.example.com/billing-eu-deploy".into(),
        sha: Some(SHA.into()),
        reading: true,
    });
    ui.settle(&app);
    assert_eq!(
        ui.session.tab, resources,
        "the Kubernetes tab until the commit shows"
    );
    app.workspace.commit = Some(groove_types::CommitEntry {
        sha: SHA.into(),
        short_sha: SHA[..7].into(),
        message: "deployed".into(),
        author: "ci".into(),
        at: groove_types::Timestamp::new(0),
        is_base: true,
    });
    app.workspace.revision = None;
    ui.settle(&app);
    assert_eq!(ui.session.tab, diff, "then its diff");
}

#[test]
fn the_in_cluster_address_is_the_application_s_own_context_with_no_kubeconfig_read() {
    let mut object = failing();
    let part = object.app.as_mut().expect("its part");
    part.destination.server = Some("https://kubernetes.default.svc".into());
    part.resources = vec![managed("ConfigMap", "billing-config", "OutOfSync", 0)];
    let (mut app, mut ui) = opened_application(object);
    let read = Command::Cluster(cluster::Command::ReadContexts);
    assert!(!asked(&app, &ui).contains(&read));
    let maps = kind("", "ConfigMap", "configmaps", true);
    let mut kinds = app.cluster.store.kinds("staging").expect("kinds").to_vec();
    kinds.push(maps.clone());
    app.cluster.store.set_kinds("staging", kinds);
    let link = Link {
        context: "staging".into(),
        kind: maps,
        namespace: Some("billing-acc".into()),
        name: "billing-config".into(),
    };
    hit(&app, &mut ui, Target::ResourceOpen(Box::new(link.clone())));
    assert_eq!(ui.session.resources.tab().map(|one| &one.link), Some(&link));
}
