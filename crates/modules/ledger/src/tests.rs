use groove_types::{Day, ExternalId};

use crate::Ledger;

async fn ledger() -> Ledger {
    Ledger::in_memory().await.expect("a ledger")
}

fn id() -> ExternalId {
    ExternalId::new("github.com/a/b#1")
}

fn today() -> Day {
    groove_types::Timestamp::now().day()
}

#[tokio::test]
async fn what_is_credited_is_tracked_and_left_to_log() {
    let ledger = ledger().await;
    ledger.credit(&id(), 90, today()).await.expect("a credit");
    ledger.credit(&id(), 30, today()).await.expect("another");
    let read = ledger.summaries().await.expect("the ledger");
    let (_, summary) = read.first().expect("one task");
    assert_eq!(summary.tracked_seconds, 120);
    assert_eq!(summary.today_seconds, 120);
    assert_eq!(summary.unlogged_seconds, 120, "nothing is logged yet");
}

#[tokio::test]
async fn what_is_logged_leaves_nothing_to_log_twice() {
    let ledger = ledger().await;
    ledger.credit(&id(), 3600, today()).await.expect("a credit");
    ledger.logged(&id(), 3600).await.expect("the hours");
    let read = ledger.summaries().await.expect("the ledger");
    let (_, summary) = read.first().expect("one task");
    assert_eq!(summary.logged_seconds, 3600);
    assert_eq!(summary.unlogged_seconds, 0);
}

#[tokio::test]
async fn a_credit_on_a_new_day_starts_today_again() {
    let ledger = ledger().await;
    let before = today().plus_days(-1);
    ledger.credit(&id(), 600, before).await.expect("yesterday");
    ledger.credit(&id(), 60, today()).await.expect("today");
    let read = ledger.summaries().await.expect("the ledger");
    let (_, summary) = read.first().expect("one task");
    assert_eq!(summary.tracked_seconds, 660, "the whole of it stands");
    assert_eq!(summary.today_seconds, 60, "today's share is today's");
}

#[tokio::test]
async fn a_task_with_nothing_measured_is_not_in_the_ledger() {
    assert!(
        ledger()
            .await
            .summaries()
            .await
            .expect("the ledger")
            .is_empty()
    );
}
