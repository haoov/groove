//! One watcher: the list a page at a time, then the watch from its version, in batches.

use std::path::PathBuf;
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use groove_kube::{Change, Client, Query};
use groove_types::{ObjectRow, TableColumn, WatchKey};
use tokio::sync::watch as signal;

/// How long changes gather before they go as one batch, and the most a batch holds.
const GATHER: Duration = Duration::from_millis(16);
const MOST: usize = 500;
const FIRST_WAIT: Duration = Duration::from_secs(1);
const LONGEST_WAIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Batch {
    /// Every row the list holds now, which replace the ones held.
    Reset {
        columns: Vec<TableColumn>,
        rows: Vec<ObjectRow>,
    },
    Changes(Vec<Delta>),
    /// Why the watcher stands still; it tries again.
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delta {
    Columns(Vec<TableColumn>),
    Put(ObjectRow),
    /// A row gone, by its uid.
    Gone(String),
}

/// What ends a watcher: stopped or dropped, the watcher ends at its next step.
#[derive(Debug)]
pub struct Stop(signal::Sender<bool>);

impl Stop {
    pub fn new() -> Self {
        Self(signal::channel(false).0)
    }

    pub fn stop(&self) {
        let _ = self.0.send(true);
    }
}

impl Default for Stop {
    fn default() -> Self {
        Self::new()
    }
}

/// Lists, then watches, `key` until `stop`; each batch goes through `send`.
pub fn watch(
    paths: Vec<PathBuf>,
    key: WatchKey,
    stop: &Stop,
    send: impl Fn(Batch) + Send + Sync + 'static,
) -> impl std::future::Future<Output = ()> + Send + 'static {
    let mut stopped = stop.0.subscribe();
    async move {
        let mut wait = FIRST_WAIT;
        loop {
            let ended = tokio::select! {
                _ = stopped.wait_for(|on| *on) => return,
                ended = cycle(&paths, &key, &send) => ended,
            };
            if let Err(why) = ended {
                send(Batch::Failed(why));
                tokio::select! {
                    _ = stopped.wait_for(|on| *on) => return,
                    _ = tokio::time::sleep(wait) => {}
                }
                wait = (wait * 2).min(LONGEST_WAIT);
            }
        }
    }
}

/// One list and the watches that follow it; `Ok` when the version expired and the list starts again.
async fn cycle(paths: &[PathBuf], key: &WatchKey, send: &impl Fn(Batch)) -> Result<(), String> {
    let client = Client::connect(paths, &key.context).await.map_err(said)?;
    let kind = crate::kube_kind(&key.kind);
    let query = Query {
        kind: &kind,
        namespace: key.namespace.as_deref(),
        selector: None,
    };
    let mut version = listed(&client, query, send).await?;
    loop {
        let opened = tokio::time::Instant::now();
        let stream = client.watch(query, &version).await.map_err(said)?;
        match watched(stream, &mut version, send).await {
            Ok(()) => tokio::time::sleep_until(opened + FIRST_WAIT).await,
            Err(groove_kube::Error::Expired { .. }) => return Ok(()),
            Err(e) => return Err(said(e)),
        }
    }
}

/// Every page, sent as one reset; returns the version the watch resumes from.
async fn listed(
    client: &Client,
    query: Query<'_>,
    send: &impl Fn(Batch),
) -> Result<String, String> {
    let mut page = client.page(query, None).await.map_err(said)?;
    let columns = page.columns.drain(..).map(crate::column_of).collect();
    let mut rows: Vec<ObjectRow> = page.rows.drain(..).map(crate::row_of).collect();
    while let Some(next) = page.next.take() {
        page = client.page(query, Some(&next)).await.map_err(said)?;
        rows.extend(page.rows.drain(..).map(crate::row_of));
    }
    send(Batch::Reset { columns, rows });
    Ok(page.version)
}

/// The changes of one watch, gathered into batches, until the server ends it.
async fn watched(
    stream: impl Stream<Item = groove_kube::Result<Vec<Change>>>,
    version: &mut String,
    send: &impl Fn(Batch),
) -> groove_kube::Result<()> {
    let mut stream = std::pin::pin!(stream);
    let mut held = Vec::new();
    loop {
        let next = match held.is_empty() {
            true => stream.next().await,
            false => match tokio::time::timeout(GATHER, stream.next()).await {
                Ok(next) => next,
                Err(_) => {
                    send(Batch::Changes(std::mem::take(&mut held)));
                    continue;
                }
            },
        };
        let Some(changes) = next else { break };
        let changes = changes.inspect_err(|_| flush(&mut held, send))?;
        for change in changes {
            held.extend(delta(change, version));
        }
        if held.len() >= MOST {
            flush(&mut held, send);
        }
    }
    flush(&mut held, send);
    Ok(())
}

fn flush(held: &mut Vec<Delta>, send: &impl Fn(Batch)) {
    if !held.is_empty() {
        send(Batch::Changes(std::mem::take(held)));
    }
}

/// The change as a delta, the version moved on to where it stands.
fn delta(change: Change, version: &mut String) -> Option<Delta> {
    match change {
        Change::Mark(at) => {
            *version = at;
            None
        }
        Change::Columns(columns) => Some(Delta::Columns(
            columns.into_iter().map(crate::column_of).collect(),
        )),
        Change::Put(row) => {
            version.clone_from(&row.version);
            Some(Delta::Put(crate::row_of(row)))
        }
        Change::Gone(uid) => Some(Delta::Gone(uid)),
    }
}

fn said(error: groove_kube::Error) -> String {
    error.to_string()
}
