use groove_types::{ExternalId, Timestamp};

use super::Timer;

fn at(seconds: i64) -> Timestamp {
    Timestamp::new(seconds)
}

fn one() -> ExternalId {
    ExternalId::new("github.com/a/b#1")
}

#[test]
fn the_ledger_takes_what_the_clock_measured_on_a_task() {
    let mut timer = Timer::default();
    timer.on(Some(one()), at(0));
    assert_eq!(timer.taken(at(60)), [(one(), 60)]);
}

#[test]
fn what_is_taken_is_not_taken_twice() {
    let mut timer = Timer::default();
    timer.on(Some(one()), at(0));
    timer.taken(at(60));
    assert_eq!(timer.taken(at(90)), [(one(), 30)], "only what came after");
}

#[test]
fn a_clock_on_nothing_owes_nothing() {
    let mut timer = Timer::default();
    timer.on(None, at(0));
    assert!(timer.taken(at(600)).is_empty());
    assert!(!timer.due(at(600), 60));
}

#[test]
fn the_run_ends_where_the_next_task_begins() {
    let (mut timer, other) = (Timer::default(), ExternalId::new("github.com/a/b#2"));
    timer.on(Some(one()), at(0));
    timer.on(Some(other.clone()), at(30));
    let taken = timer.taken(at(50));
    assert_eq!(taken, [(one(), 30), (other, 20)]);
}

#[test]
fn the_ledger_waits_its_turn_between_writes() {
    let mut timer = Timer::default();
    timer.on(Some(one()), at(0));
    assert!(timer.due(at(60), 60), "a minute of it stands");
    timer.taken(at(60));
    assert!(!timer.due(at(90), 60), "too soon after the last write");
    assert!(timer.due(at(120), 60));
}
