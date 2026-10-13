//! A session's launch files are written in the order asked, and a start waits for them.

use groove_types::SessionId;

use crate::State;

#[test]
fn a_start_asked_while_a_launch_file_write_is_out_waits_for_it() {
    let (mut state, session) = (State::default(), SessionId::new("s"));
    let _turn = state.writing(&session);
    assert_eq!(state.hold(&session, ((80, 24), None)), None, "held");
    assert_eq!(state.written(&session), Some(((80, 24), None)));
    let free = state.hold(&session, ((80, 24), None));
    assert_eq!(free, Some(((80, 24), None)), "nothing out, it runs");
}

#[test]
fn a_held_start_keeps_the_words_a_later_plain_start_lacks() {
    let (mut state, session) = (State::default(), SessionId::new("s"));
    let _turn = state.writing(&session);
    state.hold(&session, ((80, 24), Some("go".into())));
    state.hold(&session, ((100, 30), None));
    assert_eq!(
        state.written(&session),
        Some(((100, 30), Some("go".into())))
    );
}

#[tokio::test]
async fn a_session_s_launch_file_writes_run_in_the_order_asked() {
    let (mut state, session) = (State::default(), SessionId::new("s"));
    let first = state.writing(&session);
    let mut second = state.writing(&session);
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let later = order.clone();
    let waiting = tokio::spawn(async move {
        second.wait().await;
        later.lock().unwrap().push("second");
        second.end();
    });
    tokio::task::yield_now().await;
    order.lock().unwrap().push("first");
    first.end();
    waiting.await.unwrap();
    assert_eq!(*order.lock().unwrap(), ["first", "second"]);
}
