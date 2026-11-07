//! An Application's diff view: its drift read from Argo CD and drawn as the diff draws a change.

use groove_controllers::Command;
use groove_controllers::workspace::Command as Workspace;
use groove_controllers::workspace_service::prepare;

use super::argo::failing;
use super::argo_sync::{hit, opened_application};
use super::fixture::runs;
use crate::hit::Target;
use crate::tests::window;
use crate::views::session::resources::View;

fn key() -> (String, String, String) {
    ("staging".into(), "shop".into(), "billing-acc".into())
}

fn texts(app: &groove_controllers::AppState, ui: &crate::Ui) -> Vec<String> {
    runs(app, ui).into_iter().map(|one| one.text).collect()
}

#[test]
fn the_diff_view_reads_the_drift_and_draws_it_as_a_change() {
    let (mut app, mut ui) = opened_application(failing());
    hit(&app, &mut ui, Target::ResourceView(View::Diff));
    let asked = crate::frame_commands(&app, &ui, window());
    let show = Command::Workspace(Workspace::ShowDrift {
        app: key(),
        at: "1".into(),
    });
    assert!(asked.contains(&show), "{asked:?}");
    assert!(
        texts(&app, &ui)
            .iter()
            .any(|one| one.starts_with("reading the drift"))
    );

    let path = "argo/Deployment/billing-acc/billing-api.yaml".to_string();
    let pairs = vec![(path, "replicas: 2\n".into(), "replicas: 3\n".into())];
    app.workspace.drifted((key(), "1".into()), prepare(pairs));
    let shown = texts(&app, &ui);
    for one in ["replicas: 2", "replicas: 3"] {
        assert!(
            shown.iter().any(|text| text.contains(one)),
            "{one}: {shown:?}"
        );
    }
    let asked = crate::frame_commands(&app, &ui, window());
    assert!(
        !asked.contains(&show),
        "a drift shown at its version is not read again"
    );

    hit(&app, &mut ui, Target::ResourceView(View::Resources));
    let asked = crate::frame_commands(&app, &ui, window());
    assert!(
        asked.contains(&Command::Workspace(Workspace::LeaveDrift)),
        "{asked:?}"
    );
}

#[test]
fn an_application_with_nothing_differing_says_it_is_in_sync() {
    let (mut app, mut ui) = opened_application(failing());
    hit(&app, &mut ui, Target::ResourceView(View::Diff));
    app.workspace
        .drifted((key(), "1".into()), prepare(Vec::new()));
    let shown = texts(&app, &ui);
    assert!(
        shown.iter().any(|one| one.starts_with("in sync")),
        "{shown:?}"
    );
}
