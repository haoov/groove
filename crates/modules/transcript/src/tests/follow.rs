//! The follower reads the transcript from its start, then each line as it is written.

use std::time::Duration;

use groove_types::chat::Heard;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

use crate::{follow, stop};

const PROMPT: &str = r#"{"type":"user","message":{"content":"first"}}"#;
const LATER: &str = r#"{"type":"user","message":{"content":"second"}}"#;

async fn next(heard: &mut mpsc::UnboundedReceiver<Vec<Heard>>) -> Vec<Heard> {
    tokio::time::timeout(Duration::from_secs(5), heard.recv())
        .await
        .expect("a batch in time")
        .expect("the follower still runs")
}

#[tokio::test]
async fn the_follower_hears_what_is_held_then_each_whole_line_written_after() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("session.jsonl");
    tokio::fs::write(&path, format!("{PROMPT}\n"))
        .await
        .unwrap();
    let (sender, mut heard) = mpsc::unbounded_channel();
    let (held, heeded) = stop();
    let follower = tokio::spawn(follow(path.clone(), heeded, move |batch| {
        let _ = sender.send(batch);
    }));
    assert_eq!(next(&mut heard).await, [Heard::You("first".into())]);

    let mut file = tokio::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .await
        .unwrap();
    let (start, end) = LATER.split_at(10);
    file.write_all(start.as_bytes()).await.unwrap();
    file.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(400)).await;
    file.write_all(format!("{end}\n").as_bytes()).await.unwrap();
    file.flush().await.unwrap();
    assert_eq!(next(&mut heard).await, [Heard::You("second".into())]);

    drop(held);
    tokio::time::timeout(Duration::from_secs(5), follower)
        .await
        .expect("the follower ends once its stop is dropped")
        .unwrap();
}
