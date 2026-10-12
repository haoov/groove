//! One watcher: the list a page at a time, then the watch from its version, in batches.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use groove_kube::{Change, Client, Query};
use groove_types::{KubeKind, ObjectRow, TableColumn, Timestamp, WatchKey};
use tokio::sync::Notify;
use tokio::sync::watch as signal;
use tokio::time::Instant;

use crate::retry::{Backoff, FIRST_WAIT};

/// How long changes gather into a batch, the least time between batches, the most one holds.
const GATHER: Duration = Duration::from_millis(16);
const PACE: Duration = Duration::from_millis(250);
const MOST: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Batch {
    /// Every row the list holds now, by namespace then name, which replace the ones held.
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
pub struct Stop(signal::Sender<bool>, Arc<Notify>);

impl Stop {
    pub(crate) fn subscribe(&self) -> Heard {
        Heard(self.0.subscribe(), self.1.clone())
    }

    pub fn new() -> Self {
        Self(signal::channel(false).0, Arc::new(Notify::new()))
    }

    pub fn stop(&self) {
        let _ = self.0.send(true);
    }

    /// A watcher waiting to open again tries now.
    pub fn wake(&self) {
        self.1.notify_waiters();
    }
}

pub(crate) struct Heard(signal::Receiver<bool>, Arc<Notify>);

impl Heard {
    pub(crate) async fn stopped(&mut self) {
        let _ = self.0.wait_for(|on| *on).await;
    }

    /// Waits `wait`, or less when woken; false once stopped.
    pub(crate) async fn rest(&mut self, wait: Duration) -> bool {
        tokio::select! {
            _ = self.0.wait_for(|on| *on) => false,
            _ = tokio::time::sleep(wait) => true,
            _ = self.1.notified() => true,
        }
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
    let mut heard = stop.subscribe();
    async move {
        let (mut backoff, mut shown) = (Backoff::new(), false);
        loop {
            let ended = tokio::select! {
                _ = heard.stopped() => return,
                ended = cycle(&paths, &key, &send, (&mut shown, &mut backoff)) => ended,
            };
            let wait = match ended {
                Ok(()) => FIRST_WAIT,
                Err(e) => {
                    send(Batch::Failed(e.to_string()));
                    backoff.after(&e)
                }
            };
            if !heard.rest(wait).await {
                return;
            }
        }
    }
}

/// One list and the watches that follow it; `Ok` when the version expired and the list starts again.
async fn cycle(
    paths: &[PathBuf],
    key: &WatchKey,
    send: &impl Fn(Batch),
    (shown, backoff): (&mut bool, &mut Backoff),
) -> groove_kube::Result<()> {
    let client = Client::shared(paths, &key.context).await?;
    let kind = crate::kube_kind(&key.kind);
    let query = Query {
        kind: &kind,
        namespace: key.namespace.as_deref(),
        selector: key.selector.as_deref(),
        whole: crate::argo::applications(&key.kind),
    };
    let (mut version, mut columns) = listed(&client, (query, &key.kind), send, *shown).await?;
    *shown = true;
    loop {
        let opened = Instant::now();
        let stream = match client.watch(query, &version).await {
            Ok(stream) => stream,
            Err(groove_kube::Error::Expired { .. }) => return Ok(()),
            Err(e) => return Err(e),
        };
        send(Batch::Watching);
        backoff.reset();
        match watched(stream, (&key.kind, &mut version, &mut columns), send).await {
            Ok(()) => tokio::time::sleep_until(opened + FIRST_WAIT).await,
            Err(groove_kube::Error::Expired { .. }) => return Ok(()),
            Err(e) => return Err(e),
        }
    }
}

/// The first page shown at once, the rest as they come; a relist swaps in whole.
async fn listed(
    client: &Client,
    (query, kind): (Query<'_>, &KubeKind),
    send: &impl Fn(Batch),
    shown: bool,
) -> groove_kube::Result<(String, Vec<TableColumn>)> {
    let mut page = client.page(query, None).await?;
    let table: Vec<TableColumn> = page.columns.drain(..).map(crate::column_of).collect();
    let row_of = |row| crate::argo::row(kind, row, &table, Timestamp::now());
    let mut columns = Some(crate::argo::columns(kind, &table));
    let mut rows: Vec<ObjectRow> = page.rows.drain(..).map(row_of).collect();
    if !shown {
        let (columns, rows) = (
            columns.take().unwrap_or_default(),
            sorted(std::mem::take(&mut rows)),
        );
        send(Batch::Reset { columns, rows });
    }
    while let Some(next) = page.next.take() {
        page = client.page(query, Some(&next)).await?;
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

/// By namespace then name, the order the store keeps.
fn sorted(mut rows: Vec<ObjectRow>) -> Vec<ObjectRow> {
    rows.sort_by(|a, b| (&a.namespace, &a.name).cmp(&(&b.namespace, &b.name)));
    rows
}

/// The changes of one watch, gathered into batches, until the server ends it.
pub(crate) async fn watched(
    stream: impl Stream<Item = groove_kube::Result<Vec<Change>>>,
    (kind, version, columns): (&KubeKind, &mut String, &mut Vec<TableColumn>),
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
            held.extend(delta(change, kind, (version, columns)));
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
fn delta(
    change: Change,
    kind: &KubeKind,
    (version, columns): (&mut String, &mut Vec<TableColumn>),
) -> Option<Delta> {
    match change {
        Change::Mark(at) => {
            *version = at;
            None
        }
        Change::Columns(fresh) => {
            *columns = fresh.into_iter().map(crate::column_of).collect();
            Some(Delta::Columns(crate::argo::columns(kind, columns)))
        }
        Change::Put(row) => {
            version.clone_from(&row.version);
            Some(Delta::Put(crate::argo::row(
                kind,
                row,
                columns,
                Timestamp::now(),
            )))
        }
        Change::Gone(row) => Some(Delta::Gone(crate::argo::row(
            kind,
            row,
            columns,
            Timestamp::now(),
        ))),
    }
}
