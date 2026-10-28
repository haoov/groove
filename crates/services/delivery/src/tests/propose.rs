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
fn the_task_titles_it_and_the_footer_names_it() {
    let one = task(None);
    let text = text_of(Some(&one), "fix/one");
    assert_eq!(text.title, "Harden Groove");
    assert_eq!(text.body, "Task: github.com/haoov/groove#50");
}

#[test]
fn the_branch_names_it_where_there_is_no_task() {
    let text = text_of(None, "fix/one");
    assert_eq!(text.title, "fix/one");
    assert_eq!(text.body, "", "and nothing is claimed of it");
}

#[test]
fn the_footer_links_the_task_by_url_where_it_has_one() {
    let one = task(Some("https://github.com/haoov/groove/issues/50"));
    let text = text_of(Some(&one), "fix/one");
    assert_eq!(text.body, "Task: https://github.com/haoov/groove/issues/50");
}
