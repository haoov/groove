//! A kind's Table watched from a resourceVersion: each event as the change it makes to the rows.

use std::time::Duration;

use futures_util::io::AsyncBufReadExt;
use futures_util::{Stream, StreamExt};
use http::header::ACCEPT;
use serde::Deserialize;

use crate::table::{AS_TABLE, Query, raw};
use crate::{Client, Column, Error, Result, Row};

/// How long the server keeps one watch open before it ends it.
pub(crate) const TIMEOUT: &str = "290";

/// How long a watch may stay silent, longer than `TIMEOUT`.
pub(crate) const QUIET: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// The columns, which the first event of a watch carries.
    Columns(Vec<Column>),
    Put(Row),
    /// A row gone, as it last stood.
    Gone(Row),
    /// The resourceVersion the watch stands at, with no row changed.
    Mark(String),
}

impl Client {
    /// The changes since `from`, an event at a time. The stream ends when the server ends it.
    pub async fn watch(
        &self,
        query: Query<'_>,
        from: &str,
    ) -> Result<impl Stream<Item = Result<Vec<Change>>> + use<>> {
        let extra = [
            ("watch", "1"),
            ("allowWatchBookmarks", "true"),
            ("resourceVersion", from),
            ("timeoutSeconds", TIMEOUT),
        ];
        let request = http::Request::get(query.uri(&extra))
            .header(ACCEPT, AS_TABLE)
            .body(Vec::new())
            .map_err(|e| Error::Kubeconfig(e.to_string()))?;
        let context = self.context.clone();
        let lines = self.lines(request, Some(QUIET)).await?;
        Ok(lines.map(move |line| line.and_then(|line| changes(&context, &line))))
    }

    /// A watch's lines as they come, the blank ones skipped, until the response ends or `quiet` passes.
    pub(crate) async fn lines(
        &self,
        request: http::Request<Vec<u8>>,
        quiet: Option<Duration>,
    ) -> Result<impl Stream<Item = Result<String>> + use<>> {
        let context = self.context.clone();
        let opened = self
            .answered(async {
                let asked = self.inner.request_stream(request).await;
                asked.map_err(|e| Error::of(&context, e))
            })
            .await?;
        let lines = silenced(opened.lines(), quiet, context.clone());
        Ok(lines.filter_map(move |line| {
            let read = match line {
                Ok(line) if line.trim().is_empty() => None,
                Ok(line) => Some(Ok(line)),
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => None,
                Err(e) => Some(Err(Error::Unreachable {
                    context: context.clone(),
                    detail: e.to_string(),
                })),
            };
            std::future::ready(read)
        }))
    }
}

/// The stream, ended by an error once `quiet` passes with nothing from it.
pub(crate) fn silenced<T>(
    stream: impl Stream<Item = std::io::Result<T>> + Unpin,
    quiet: Option<Duration>,
    context: String,
) -> impl Stream<Item = std::io::Result<T>> {
    futures_util::stream::unfold(Some(stream), move |held| {
        let context = context.clone();
        async move {
            let mut stream = held?;
            let next = match quiet {
                Some(long) => tokio::time::timeout(long, stream.next()).await,
                None => Ok(stream.next().await),
            };
            match next {
                Ok(Some(item)) => Some((item, Some(stream))),
                Ok(None) => None,
                Err(_) => {
                    crate::pool::forget(&context);
                    let secs = quiet.unwrap_or_default().as_secs();
                    let silent = std::io::Error::other(format!("no answer in {secs} s"));
                    Some((Err(silent), None))
                }
            }
        }
    })
}

/// One line of a watch, read in one pass; a bookmark's object stays loose, its shape varies.
#[derive(Deserialize)]
#[serde(tag = "type", content = "object", rename_all = "UPPERCASE")]
enum Event {
    Added(raw::Table),
    Modified(raw::Table),
    Deleted(raw::Table),
    Bookmark(serde_json::Value),
    Error(kube::core::Status),
}

fn changes(context: &str, line: &str) -> Result<Vec<Change>> {
    let unreadable = |detail: String| Error::Unreadable {
        context: context.to_string(),
        detail,
    };
    let event: Event = serde_json::from_str(line).map_err(|e| unreadable(e.to_string()))?;
    let (table, gone) = match event {
        Event::Added(table) | Event::Modified(table) => (table, false),
        Event::Deleted(table) => (table, true),
        Event::Bookmark(object) => return Ok(marked(&object).into_iter().collect()),
        Event::Error(status) => return Err(Error::of(context, kube::Error::Api(status.boxed()))),
    };
    let mut out = Vec::new();
    if !table.columns.is_empty() {
        out.push(Change::Columns(
            table.columns.into_iter().map(Column::from).collect(),
        ));
    }
    for row in table.rows.into_iter().map(Row::from) {
        out.push(match gone {
            true => Change::Gone(row),
            false => Change::Put(row),
        });
    }
    Ok(out)
}

/// A bookmark's version: on the Table itself, or on its one row when the server converted it.
fn marked(object: &serde_json::Value) -> Option<Change> {
    let on_table = object.pointer("/metadata/resourceVersion");
    let on_row = object.pointer("/rows/0/object/metadata/resourceVersion");
    let version = on_table
        .or(on_row)?
        .as_str()
        .filter(|one| !one.is_empty())?;
    Some(Change::Mark(version.to_string()))
}
