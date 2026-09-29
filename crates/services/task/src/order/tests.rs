use groove_plan::Placed;
use groove_types::{ExternalId, ProviderId, Task, TaskDates, Timestamp};

use super::{moved, ordered};

fn task(id: &str) -> Task {
    Task {
        external_id: ExternalId::new(id),
        short_id: id.to_string(),
        title: id.to_string(),
        status: "Todo".into(),
        intent: None,
        priority: None,
        dates: TaskDates::default(),
        estimate: None,
        logged: None,
        synced_at: Timestamp::new(0),
        provider: ProviderId::Github,
        url: None,
        project: None,
        branch_tag: None,
    }
}

fn placed(id: &str, later: bool) -> Placed {
    Placed {
        external_id: ExternalId::new(id),
        later,
    }
}

fn names<'a>(planned: &[super::Planned<'a>]) -> Vec<(&'a str, bool)> {
    planned
        .iter()
        .map(|one| (one.task.short_id.as_str(), one.later))
        .collect()
}

#[test]
fn the_plan_puts_the_tasks_it_places_before_the_ones_it_does_not() {
    let (a, b, c) = (task("a"), task("b"), task("c"));
    let tasks = [&a, &b, &c];
    let order = [placed("c", false), placed("b", false)];
    let planned = ordered(&order, &tasks);
    assert_eq!(names(&planned), [("c", false), ("b", false), ("a", false)]);
}

#[test]
fn a_task_under_the_divider_stands_below_every_task_over_it() {
    let (a, b, c) = (task("a"), task("b"), task("c"));
    let tasks = [&a, &b, &c];
    let order = [placed("a", true), placed("b", false)];
    let planned = ordered(&order, &tasks);
    assert_eq!(names(&planned), [("b", false), ("c", false), ("a", true)]);
}

#[test]
fn a_placed_task_the_sources_no_longer_answer_with_is_left_out() {
    let a = task("a");
    let tasks = [&a];
    let order = [placed("gone", false), placed("a", false)];
    assert_eq!(names(&ordered(&order, &tasks)), [("a", false)]);
}

#[test]
fn a_move_puts_the_task_above_the_one_it_lands_on() {
    let (a, b, c) = (task("a"), task("b"), task("c"));
    let tasks = [&a, &b, &c];
    let shown = ordered(&[], &tasks);
    let order = moved(&shown, &c.external_id, Some(&a.external_id), false);
    assert_eq!(
        order,
        [placed("c", false), placed("a", false), placed("b", false)]
    );
}

#[test]
fn a_move_with_nothing_below_it_ends_the_side_it_lands_on() {
    let (a, b) = (task("a"), task("b"));
    let tasks = [&a, &b];
    let shown = ordered(&[placed("a", false), placed("b", true)], &tasks);
    let order = moved(&shown, &a.external_id, None, false);
    assert_eq!(
        order,
        [placed("a", false), placed("b", true)],
        "the end of its own side, over the divider"
    );
    let order = moved(&shown, &a.external_id, None, true);
    assert_eq!(
        order,
        [placed("b", true), placed("a", true)],
        "and under it when that is where it was dropped"
    );
}
