//! One stream of a pod's logs: its containers merged, gathered into batches, picked up where it broke.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use groove_kube::{Client, LogQuery, Start};
use groove_types::{LogKey, LogLine, LogRange, LogSource, LogTime};
use tokio::time::Instant;

use crate::Stop;

/// How long a batch gathers lines once the first arrives, and how many it holds at most.
const PACE: Duration = Duration::from_millis(100);
const MOST: usize = 5000;
const FIRST_WAIT: Duration = Duration::from_secs(1);
const LONGEST_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub enum Logged {
    Lines(Vec<LogLine>),
    /// The streams are open.
    Streaming,
    /// Every stream ended: the container stopped, or the log was read to its end.
    Ended,
    Failed(String),
}

/// Streams `key` until `stop`; each batch goes through `send`. A followed stream that ends opens again.
pub fn logs(
    paths: Vec<PathBuf>,
    key: LogKey,
    stop: &Stop,
    send: impl Fn(Logged) + Send + Sync + 'static,
) -> impl std::future::Future<Output = ()> + Send + 'static {
    let mut stopped = stop.subscribe();
    async move {
        let mut last: BTreeMap<String, Seen> = BTreeMap::new();
        let mut wait = FIRST_WAIT;
        loop {
            let ended = tokio::select! {
                _ = stopped.wait_for(|on| *on) => return,
                ended = cycle(&paths, &key, &mut last, &send) => ended,
            };
            let pause = match ended {
                Ok(()) if key.previous => return send(Logged::Ended),
                Ok(()) => {
                    send(Logged::Ended);
                    wait = FIRST_WAIT;
                    FIRST_WAIT
                }
                Err(why) => {
                    send(Logged::Failed(why));
                    let pause = wait;
                    wait = (wait * 2).min(LONGEST_WAIT);
                    pause
                }
            };
            tokio::select! {
                _ = stopped.wait_for(|on| *on) => return,
                _ = tokio::time::sleep(pause) => {}
            }
        }
    }
}

/// The streams of every container read, merged, until they all end.
async fn cycle(
    paths: &[PathBuf],
    key: &LogKey,
    last: &mut BTreeMap<String, Seen>,
    send: &impl Fn(Logged),
) -> Result<(), String> {
    let client = Client::connect(paths, &key.context).await.map_err(said)?;
    let containers = match &key.source {
        LogSource::Container(name) => vec![name.clone()],
        LogSource::All => containers(&client, key).await?,
    };
    let tagged = matches!(key.source, LogSource::All);
    let mut streams = Vec::new();
    for container in &containers {
        let start = start(key.range, last.get(container));
        let query = LogQuery {
            namespace: &key.namespace,
            pod: &key.pod,
            container,
            previous: key.previous,
            follow: !key.previous,
            start: &start,
        };
        let lines = client.logs(query).await.map_err(said)?;
        let name = container.clone();
        streams.push(lines.map(move |line| (name.clone(), line)).boxed());
    }
    send(Logged::Streaming);
    gathered(
        futures_util::stream::select_all(streams),
        (tagged, last),
        send,
    )
    .await
}

/// The last line a container's stream gave: its time, and its stamp cut to the second.
type Seen = (LogTime, String);

/// Where a stream starts: the range asked the first time, the last line's second after a break.
fn start(range: LogRange, last: Option<&Seen>) -> Start {
    if let Some((_, second)) = last {
        return Start::Time(second.clone());
    }
    match range {
        LogRange::Since(seconds) => Start::Seconds(seconds),
        LogRange::All => Start::Beginning,
    }
}

/// Lines gathered for `PACE` after the first of a batch; a line no newer than the last seen is a repeat.
async fn gathered(
    stream: impl Stream<Item = (String, groove_kube::Result<String>)>,
    (tagged, last): (bool, &mut BTreeMap<String, Seen>),
    send: &impl Fn(Logged),
) -> Result<(), String> {
    let mut stream = std::pin::pin!(stream);
    let mut held: Vec<LogLine> = Vec::new();
    let mut due: Option<Instant> = None;
    loop {
        let next = match due {
            None => stream.next().await,
            Some(at) => match tokio::time::timeout_at(at, stream.next()).await {
                Ok(next) => next,
                Err(_) => {
                    send(Logged::Lines(std::mem::take(&mut held)));
                    due = None;
                    continue;
                }
            },
        };
        let Some((container, line)) = next else { break };
        let line = LogLine::read(&line.map_err(said)?, tagged.then(|| container.clone()));
        if let Some(at) = line.at {
            if last.get(&container).is_some_and(|(seen, _)| at <= *seen) {
                continue;
            }
            let second = line.stamp.split('.').next().unwrap_or_default();
            let second = format!("{}Z", second.trim_end_matches('Z'));
            last.insert(container, (at, second));
        }
        held.push(line);
        due = due.or_else(|| Some(Instant::now() + PACE));
        if held.len() >= MOST {
            send(Logged::Lines(std::mem::take(&mut held)));
            due = None;
        }
    }
    if !held.is_empty() {
        send(Logged::Lines(held));
    }
    Ok(())
}

/// The pod's containers, its init ones first.
async fn containers(client: &Client, key: &LogKey) -> Result<Vec<String>, String> {
    let path = format!("/api/v1/namespaces/{}/pods/{}", key.namespace, key.pod);
    let pod = client.get_object(&path).await.map_err(said)?;
    let names = |at: &str| {
        let listed = pod.pointer(at).and_then(serde_json::Value::as_array);
        let named = listed.into_iter().flatten();
        named
            .filter_map(|one| one["name"].as_str().map(String::from))
            .collect::<Vec<_>>()
    };
    Ok([names("/spec/initContainers"), names("/spec/containers")].concat())
}

fn said(error: groove_kube::Error) -> String {
    error.to_string()
}
