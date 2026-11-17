//! A maintenance task's overview: the flag, its children; every task's log.

use groove_types::{LogEntry, SessionId, Timestamp};

use crate::tests::{metrics, open, texts_of};
use crate::{Ui, view};

fn texts(app: &groove_controllers::AppState) -> Vec<String> {
    let (frame, _) = view(
        app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    texts_of(&frame)
}

#[test]
fn a_maintenance_task_lists_its_children_and_the_log_shows_who_wrote_each_entry() {
    let mut app = super::working_a_task();
    let parent = app.session.get_mut(&SessionId::new("a")).expect("the task");
    parent.state.maintenance = true;
    parent.children = vec![open("child-1", "VeleroBackupFailed").session];
    parent.log = vec![LogEntry {
        from: Some("KubeJobFailed".into()),
        what: "Raised the backup job's limit to 2Gi.".into(),
        why: Some("It ran out of memory.".into()),
        mr_url: None,
        at: Timestamp::new(0),
    }];
    let said = texts(&app);
    for one in [
        "MAINTENANCE",
        "CHILDREN",
        "VeleroBackupFailed",
        "closed",
        "LOG",
        "Raised the backup job's limit to 2Gi.",
        "It ran out of memory.",
        "from KubeJobFailed",
    ] {
        assert!(said.iter().any(|text| text == one), "{one}: {said:?}");
    }
}

#[test]
fn a_task_not_marked_shows_no_children_and_an_empty_log() {
    let said = texts(&super::working_a_task());
    assert!(!said.iter().any(|text| text == "CHILDREN"), "{said:?}");
    assert!(said.iter().any(|text| text == "Nothing logged yet."), "{said:?}");
}
