//! One watcher of whole objects: listed, then watched, each read into what its tab draws.

use std::path::PathBuf;
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use groove_kube::{Client, Narrowed, Seen};
use groove_types::{Described, FollowKey};
use tokio::time::Instant;

use crate::Stop;
use crate::retry::{Backoff, FIRST_WAIT};

const GATHER: Duration = Duration::from_millis(16);

#[derive(Debug, Clone, PartialEq)]
pub enum Followed {
    /// Every object the list holds now, which replace the ones held.
    Reset(Vec<Described>),
    Changes(Vec<Change>),
    Watching,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Put(Box<Described>),
    /// The uid of an object gone.
    Gone(String),
}

/// Lists, then watches, `key` until `stop`; each batch goes through `send`.
pub fn follow(
    paths: Vec<PathBuf>,
    key: FollowKey,
    stop: &Stop,
    send: impl Fn(Followed) + Send + Sync + 'static,
) -> impl std::future::Future<Output = ()> + Send + 'static {
    let mut heard = stop.subscribe();
    async move {
        let mut backoff = Backoff::new();
        loop {
            let ended = tokio::select! {
                _ = heard.stopped() => return,
                ended = cycle(&paths, &key, &send, &mut backoff) => ended,
            };
            let wait = match ended {
                Ok(()) => FIRST_WAIT,
                Err(e) => {
                    send(Followed::Failed(e.to_string()));
                    backoff.after(&e)
                }
            };
            if !heard.rest(wait).await {
                return;
            }
        }
    }
}

/// One list and the watches after it; `Ok` when the version expired and the list starts again.
async fn cycle(
    paths: &[PathBuf],
    key: &FollowKey,
    send: &impl Fn(Followed),
    backoff: &mut Backoff,
) -> groove_kube::Result<()> {
    let client = Client::shared(paths, &key.context).await?;
    let kind = crate::kube_kind(&key.kind);
    let narrowed = Narrowed {
        kind: &kind,
        namespace: key.namespace.as_deref(),
        fields: key.fields.as_deref(),
        labels: None,
    };
    let listed = client.list_objects(narrowed).await?;
    let whole = key
        .fields
        .as_deref()
        .is_some_and(|one| one.starts_with("metadata.name="));
    let read = |object: &serde_json::Value| crate::describe(&key.kind.kind, object, whole);
    send(Followed::Reset(listed.objects.iter().map(read).collect()));
    let mut version = listed.version;
    loop {
        let opened = Instant::now();
        let stream = match client.watch_objects(narrowed, &version).await {
            Ok(stream) => stream,
            Err(groove_kube::Error::Expired { .. }) => return Ok(()),
            Err(e) => return Err(e),
        };
        send(Followed::Watching);
        backoff.reset();
        match watched(stream, &mut version, &read, send).await {
            Ok(()) => tokio::time::sleep_until(opened + FIRST_WAIT).await,
            Err(groove_kube::Error::Expired { .. }) => return Ok(()),
            Err(e) => return Err(e),
        }
    }
}

/// The changes of one watch, gathered for a moment into each batch, until the server ends it.
async fn watched(
    stream: impl Stream<Item = groove_kube::Result<Seen>>,
    version: &mut String,
    read: &impl Fn(&serde_json::Value) -> Described,
    send: &impl Fn(Followed),
) -> groove_kube::Result<()> {
    let mut stream = std::pin::pin!(stream);
    let mut held = Vec::new();
    loop {
        let next = match held.is_empty() {
            true => stream.next().await,
            false => match tokio::time::timeout(GATHER, stream.next()).await {
                Ok(next) => next,
                Err(_) => {
                    send(Followed::Changes(std::mem::take(&mut held)));
                    continue;
                }
            },
        };
        let Some(seen) = next else { break };
        match seen? {
            Seen::Mark(at) if !at.is_empty() => *version = at,
            Seen::Mark(_) => {}
            Seen::Put(object) => {
                let at = crate::describe::text(&object, "/metadata/resourceVersion");
                if !at.is_empty() {
                    *version = at;
                }
                held.push(Change::Put(Box::new(read(&object))));
            }
            Seen::Gone(object) => held.push(Change::Gone(crate::describe::text(
                &object,
                "/metadata/uid",
            ))),
        }
    }
    if !held.is_empty() {
        send(Followed::Changes(held));
    }
    Ok(())
}
