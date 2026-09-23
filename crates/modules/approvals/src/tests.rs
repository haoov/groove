use groove_types::{Approval, Origin, SessionId, Timestamp};

use crate::{New, Queue};

fn new(session: &str, op: &str) -> New {
    New {
        session: SessionId::new(session),
        op: op.to_string(),
        payload: serde_json::json!({ "message": "fix: one" }),
        origin: Origin::Mcp,
        at: Timestamp::new(10),
    }
}

fn said(one: &Approval) -> String {
    one.payload["message"].as_str().unwrap_or_default().into()
}

#[test]
fn a_write_waits_until_it_is_resolved() {
    let mut queue: Queue<u32> = Queue::default();
    let approval = queue.queue(new("a", "git_commit"), 7);
    assert_eq!(queue.len(), 1);
    let (taken, held) = queue.resolve(&approval.id).expect("the write");
    assert_eq!(held, 7);
    assert_eq!(taken.op, "git_commit");
    assert!(queue.is_empty(), "it is decided once");
    assert!(queue.resolve(&approval.id).is_none());
}

#[test]
fn no_two_writes_share_an_id() {
    let mut queue: Queue<u32> = Queue::default();
    let one = queue.queue(new("a", "git_commit"), 1);
    let two = queue.queue(new("a", "git_push"), 2);
    assert_ne!(one.id, two.id);
}

#[test]
fn a_session_sees_its_own_writes_and_no_others() {
    let mut queue: Queue<u32> = Queue::default();
    queue.queue(new("a", "git_commit"), 1);
    queue.queue(new("b", "git_push"), 2);
    let asks = queue.asks(&SessionId::new("a"), said);
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].op, "git_commit");
    assert_eq!(asks[0].subject, "fix: one");
}

#[test]
fn a_session_that_closes_leaves_nothing_waiting() {
    let mut queue: Queue<u32> = Queue::default();
    queue.queue(new("a", "git_commit"), 1);
    queue.queue(new("a", "git_push"), 2);
    queue.queue(new("b", "git_push"), 3);
    let dropped = queue.forget(&SessionId::new("a"));
    assert_eq!(dropped.len(), 2, "both are handed back to be failed");
    assert_eq!(queue.len(), 1);
}
