use std::sync::{Arc, Mutex};

use crate::{Ended, Turn, lock};

#[test]
fn a_lock_a_panic_left_poisoned_still_opens() {
    let held = Arc::new(Mutex::new(1));
    let poisoner = held.clone();
    let _ = std::thread::spawn(move || {
        let _guard = poisoner.lock();
        panic!("poisoned");
    })
    .join();
    assert_eq!(*lock(&held), 1);
}

#[tokio::test]
async fn a_turn_waits_for_the_one_before_it_to_end() {
    let mut last: Option<Ended> = None;
    let first = Turn::behind(|end| last.replace(end));
    let mut second = Turn::behind(|end| last.replace(end));
    let waiting = tokio::spawn(async move {
        second.wait().await;
        "ran"
    });
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished(), "held back by the first");
    first.end();
    assert_eq!(waiting.await.unwrap(), "ran");
}
