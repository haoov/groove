//! A maintenance task's overview: the flag and its children.

use groove_types::SessionId;

use crate::tests::{all_texts, metrics, open};
use crate::view;

fn texts(app: &groove_controllers::AppState) -> Vec<String> {
    let (frame, _) = view(
        app,
        &super::overviewed(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    all_texts(&frame)
}

#[test]
fn a_maintenance_task_lists_its_children() {
    let mut app = super::working_a_task();
    let parent = app.session.get_mut(&SessionId::new("a")).expect("the task");
    parent.state.maintenance = true;
    parent.children = vec![open("child-1", "VeleroBackupFailed").session];
    let said = texts(&app);
    for one in ["MAINTENANCE", "CHILDREN · 1", "VeleroBackup", "closed"] {
        assert!(
            said.iter().any(|text| text.starts_with(one)),
            "{one}: {said:?}"
        );
    }
}

#[test]
fn a_task_not_marked_shows_no_children() {
    let said = texts(&super::working_a_task());
    assert!(
        !said.iter().any(|text| text.starts_with("CHILDREN")),
        "{said:?}"
    );
}
