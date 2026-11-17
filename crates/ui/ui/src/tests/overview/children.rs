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
        id: 1,
        from: Some("KubeJobFailed".into()),
        what: "Raised the backup job's limit to 2Gi.".into(),
        why: Some("It ran out of memory.".into()),
        mr_url: None,
        at: Timestamp::new(0),
    }];
    let said = texts(&app);
    for one in [
        "MAINTENANCE",
        "CHILDREN · 1",
        "VeleroBackupFailed",
        "closed",
        "LOG · 1",
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
    assert!(
        !said.iter().any(|text| text.starts_with("CHILDREN")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|text| text == "Nothing logged yet."),
        "{said:?}"
    );
}

#[test]
fn an_entry_s_button_removes_it_from_the_log() {
    use groove_controllers::{Command, session};
    let mut app = super::working_a_task();
    let task = app.session.get_mut(&SessionId::new("a")).expect("the task");
    task.log = vec![LogEntry {
        id: 7,
        from: None,
        what: "Logged by mistake".into(),
        why: None,
        mr_url: None,
        at: Timestamp::new(0),
    }];
    let (_, hits) = view(
        &app,
        &Ui::default(),
        metrics(1280, 800, 1.0),
        &mut groove_gfx::Fonts::embedded(),
    );
    let button = hits
        .rect_of(&crate::hit::Target::LogRemove(7))
        .expect("the remove button");
    let mut ui = Ui::default();
    let clicked = crate::tests::click(button, &mut ui, &app, &hits);
    let remove = session::Command::RemoveLogEntry {
        session: SessionId::new("a"),
        entry: 7,
    };
    assert_eq!(clicked, [Command::Session(remove)]);
}
