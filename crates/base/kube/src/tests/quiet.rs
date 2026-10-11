use futures_util::StreamExt;

use crate::watch::{QUIET, silenced};

#[tokio::test(start_paused = true)]
async fn a_watch_silent_past_its_end_fails_once_and_stops() {
    let silent = futures_util::stream::pending::<std::io::Result<String>>();
    let read: Vec<_> = silenced(silent, Some(QUIET), "kind".into()).collect().await;
    assert_eq!(read.len(), 1);
    let said = read[0].as_ref().expect_err("a failure").to_string();
    assert_eq!(said, "no answer in 300 s");
}

#[tokio::test(start_paused = true)]
async fn a_log_waits_on_its_lines_as_long_as_they_take() {
    let late = futures_util::stream::once(async {
        tokio::time::sleep(QUIET * 10).await;
        Ok::<_, std::io::Error>("late".to_string())
    });
    let read: Vec<_> = silenced(late.boxed(), None, "kind".into()).collect().await;
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].as_ref().expect("the line"), "late");
}
