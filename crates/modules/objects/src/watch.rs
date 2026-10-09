//! One watcher: the list a page at a time, then the watch from its version, in batches.

use std::path::PathBuf;
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use groove_kube::{Change, Client, Query};
use groove_types::{ObjectRow, TableColumn, Timestamp, WatchKey};
use tokio::sync::watch as signal;
use tokio::time::Instant;

/// How long changes gather into a batch, the least time between batches, the most one holds.
const GATHER: Duration = Duration::from_millis(16);
const PACE: Duration = Duration::from_millis(250);
const MOST: usize = 10_000;
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
    /// A watch is open: changes arrive as they happen.
    Watching,
    /// Why the watcher stands still; it tries again.
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delta {
    Columns(Vec<TableColumn>),
    Put(ObjectRow),
    /// A row gone, as it last stood.
    Gone(ObjectRow),
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
        let (mut wait, mut shown) = (FIRST_WAIT, false);
        loop {
            let ended = tokio::select! {
                _ = stopped.wait_for(|on| *on) => return,
                ended = cycle(&paths, &key, &send, &mut shown) => ended,
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
async fn cycle(
    paths: &[PathBuf],
    key: &WatchKey,
    send: &impl Fn(Batch),
    shown: &mut bool,
) -> Result<(), String> {
    let client = Client::connect(paths, &key.context).await.map_err(said)?;
    let kind = crate::kube_kind(&key.kind);
    let query = Query {
        kind: &kind,
        namespace: key.namespace.as_deref(),
        selector: key.selector.as_deref(),
    };
    let (mut version, mut columns) = listed(&client, query, send, *shown).await?;
    *shown = true;
    loop {
        let opened = Instant::now();
        let stream = match client.watch(query, &version).await {
            Ok(stream) => stream,
            Err(groove_kube::Error::Expired { .. }) => return Ok(()),
            Err(e) => return Err(said(e)),
        };
        send(Batch::Watching);
        match watched(stream, (&mut version, &mut columns), send).await {
            Ok(()) => tokio::time::sleep_until(opened + FIRST_WAIT).await,
            Err(groove_kube::Error::Expired { .. }) => return Ok(()),
            Err(e) => return Err(said(e)),
        }
    }
}

/// The first page shown at once, the rest as they come; a relist swaps in whole.
async fn listed(
    client: &Client,
    query: Query<'_>,
    send: &impl Fn(Batch),
    shown: bool,
) -> Result<(String, Vec<TableColumn>), String> {
    let mut page = client.page(query, None).await.map_err(said)?;
    let table: Vec<TableColumn> = page.columns.drain(..).map(crate::column_of).collect();
    let row_of = |row| crate::row_of(row, &table, Timestamp::now());
    let mut columns = Some(table.clone());
    let mut rows: Vec<ObjectRow> = page.rows.drain(..).map(row_of).collect();
    if !shown {
        let (columns, rows) = (
            columns.take().unwrap_or_default(),
            sorted(std::mem::take(&mut rows)),
        );
        send(Batch::Reset { columns, rows });
    }
    while let Some(next) = page.next.take() {
        page = client.page(query, Some(&next)).await.map_err(said)?;
        let more = page.rows.drain(..).map(row_of);
        match shown {
            true => rows.extend(more),
            false => send(Batch::Changes(more.map(Delta::Put).collect())),
        }
    }
    if let Some(columns) = columns {
        send(Batch::Reset {
            columns,
            rows: sorted(rows),
        });
    }
    Ok((page.version, table))
}

/// By namespace then name, the order the store keeps, so the main thread sorts nothing.
fn sorted(mut rows: Vec<ObjectRow>) -> Vec<ObjectRow> {
    rows.sort_by(|a, b| (&a.namespace, &a.name).cmp(&(&b.namespace, &b.name)));
    rows
}

/// The changes of one watch, gathered into batches, until the server ends it.
pub(crate) async fn watched(
    stream: impl Stream<Item = groove_kube::Result<Vec<Change>>>,
    (version, columns): (&mut String, &mut Vec<TableColumn>),
    send: &impl Fn(Batch),
) -> groove_kube::Result<()> {
    let mut stream = std::pin::pin!(stream);
    let mut held = Vec::new();
    let mut sent = Instant::now()
        .checked_sub(PACE)
        .unwrap_or_else(Instant::now);
    let mut due = Instant::now();
    loop {
        let next = match held.is_empty() {
            true => stream.next().await,
            false => match tokio::time::timeout_at(due, stream.next()).await {
                Ok(next) => next,
                Err(_) => {
                    flush(&mut held, send);
                    sent = Instant::now();
                    continue;
                }
            },
        };
        let Some(changes) = next else { break };
        let changes = changes.inspect_err(|_| flush(&mut held, send))?;
        if held.is_empty() {
            due = (Instant::now() + GATHER).max(sent + PACE);
        }
        for change in changes {
            held.extend(delta(change, version, columns));
        }
        if held.len() >= MOST {
            flush(&mut held, send);
            sent = Instant::now();
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

/// The change as a delta, the version and the columns moved on to where they stand.
fn delta(change: Change, version: &mut String, columns: &mut Vec<TableColumn>) -> Option<Delta> {
    match change {
        Change::Mark(at) => {
            *version = at;
            None
        }
        Change::Columns(fresh) => {
            *columns = fresh.into_iter().map(crate::column_of).collect();
            Some(Delta::Columns(columns.clone()))
        }
        Change::Put(row) => {
            version.clone_from(&row.version);
            Some(Delta::Put(crate::row_of(row, columns, Timestamp::now())))
        }
        Change::Gone(row) => Some(Delta::Gone(crate::row_of(row, columns, Timestamp::now()))),
    }
}

fn said(error: groove_kube::Error) -> String {
    error.to_string()
}
