//! What an MR is opened with: its title, its body, and the task it names.

use groove_types::{ExternalId, ProviderId, Task, TaskDates, Timestamp};

use crate::text_of;

fn task(url: Option<&str>) -> Task {
    Task {
        external_id: ExternalId::new("github.com/haoov/groove#50"),
        short_id: "gh-haoov-groove-50".into(),
        title: "Harden Groove".into(),
        status: "In progress".into(),
        intent: None,
        priority: None,
        dates: TaskDates::default(),
        estimate: None,
        logged: None,
        synced_at: Timestamp::new(0),
        provider: ProviderId::Github,
        url: url.map(str::to_string),
        project: None,
        branch_tag: None,
    }
}

#[test]
fn the_message_titles_it_and_the_rest_is_its_body() {
    let message = "feat(forge): open a merge request\n\nThe commit box says what it is.\n";
    let text = text_of(message, None, "fix/one");
    assert_eq!(text.title, "feat(forge): open a merge request");
    assert_eq!(text.body, "The commit box says what it is.");
}

#[test]
fn an_empty_box_takes_the_tasks_own_title() {
    let one = task(None);
    let text = text_of("", Some(&one), "fix/one");
    assert_eq!(text.title, "Harden Groove");
    assert_eq!(text.body, "Task: github.com/haoov/groove#50");
}

#[test]
fn the_branch_names_it_where_there_is_no_task_and_no_message() {
    let text = text_of("  \n", None, "fix/one");
    assert_eq!(text.title, "fix/one");
    assert_eq!(text.body, "", "and nothing is claimed of it");
}

#[test]
fn the_footer_links_the_task_by_url_where_it_has_one() {
    let one = task(Some("https://github.com/haoov/groove/issues/50"));
    let text = text_of("fix: one\n\nwhy it is\n", Some(&one), "fix/one");
    assert_eq!(
        text.body,
        "why it is\n\nTask: https://github.com/haoov/groove/issues/50"
    );
}
