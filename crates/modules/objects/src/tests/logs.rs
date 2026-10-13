use std::sync::{Arc, Mutex};
use std::time::Duration;

use groove_types::{LogKey, LogRange, LogSource};
use wiremock::matchers::{path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::watch::kubeconfig;
use crate::{Logged, Stop, logs};

fn worker(range: LogRange) -> LogKey {
    LogKey {
        context: "kind".into(),
        namespace: "paxone".into(),
        pod: "api-0".into(),
        source: LogSource::Container("worker".into()),
        previous: false,
        range,
    }
}

#[tokio::test]
async fn a_stream_that_ends_opens_again_from_its_last_second_and_drops_the_lines_it_had() {
    let server = MockServer::start().await;
    let log = "/api/v1/namespaces/paxone/pods/api-0/log";
    Mock::given(path(log))
        .and(query_param("sinceSeconds", "300"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "2026-10-09T14:02:09.100Z ready\n2026-10-09T14:02:10.200Z job 1\n",
            ),
        )
        .mount(&server)
        .await;
    Mock::given(path(log))
        .and(query_param("sinceTime", "2026-10-09T14:02:10Z"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                "2026-10-09T14:02:10.200Z job 1\n2026-10-09T14:02:11.300Z job 2\n",
            ),
        )
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = vec![kubeconfig(dir.path(), &server.uri())];
    let sent = Arc::new(Mutex::new(Vec::new()));
    let held = sent.clone();
    let stop = Stop::new();
    let follower = logs(paths, worker(LogRange::Since(300)), &stop, move |batch| {
        held.lock().expect("the batches").push(batch);
    });
    let running = tokio::spawn(follower);
    tokio::time::sleep(Duration::from_millis(1500)).await;
    stop.stop();
    running.await.expect("the follower ends");
    let sent = sent.lock().expect("the batches").clone();
    let words: Vec<String> = sent
        .iter()
        .filter_map(|one| match one {
            Logged::Lines(lines) => Some(lines.iter().map(|line| line.text.clone())),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(words, ["ready", "job 1", "job 2"], "{sent:?}");
    assert!(sent.contains(&Logged::Ended), "{sent:?}");
}

#[tokio::test]
async fn a_stream_that_ends_with_nothing_new_waits_longer_each_time() {
    let server = MockServer::start().await;
    Mock::given(path("/api/v1/namespaces/paxone/pods/api-0/log"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = vec![kubeconfig(dir.path(), &server.uri())];
    let stop = Stop::new();
    let running = tokio::spawn(logs(paths, worker(LogRange::Since(300)), &stop, |_| {}));
    tokio::time::sleep(Duration::from_millis(2_500)).await;
    stop.stop();
    running.await.expect("the follower ends");
    let asked = server.received_requests().await.expect("recorded");
    assert_eq!(
        asked.len(),
        2,
        "at once, then after 1 s; the next after 2 s more"
    );
}

#[tokio::test]
async fn a_stream_parked_on_a_missing_context_tries_again_when_woken() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let paths = vec![kubeconfig(dir.path(), "http://127.0.0.1:9")];
    let failed = Arc::new(Mutex::new(0));
    let counted = failed.clone();
    let stop = Stop::new();
    let key = LogKey {
        context: "gone".into(),
        ..worker(LogRange::All)
    };
    let running = tokio::spawn(logs(paths, key, &stop, move |batch| {
        if matches!(batch, Logged::Failed(_)) {
            *counted.lock().expect("the count") += 1;
        }
    }));
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(*failed.lock().expect("the count"), 1);
    stop.wake();
    tokio::time::sleep(Duration::from_millis(200)).await;
    stop.stop();
    running.await.expect("the stream ends");
    assert_eq!(*failed.lock().expect("the count"), 2);
}
