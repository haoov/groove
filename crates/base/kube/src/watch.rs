//! A kind's Table watched from a resourceVersion: each event as the change it makes to the rows.

use futures_util::{Stream, StreamExt};
use http::header::ACCEPT;
use kube::core::WatchEvent;

use crate::table::{AS_TABLE, Query, raw};
use crate::{Client, Column, Error, Result, Row};

/// How long the server keeps one watch open before it ends it.
const TIMEOUT: &str = "290";

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
        let events = self
            .inner
            .request_events::<raw::Table>(request)
            .await
            .map_err(|e| Error::of(&context, e))?;
        Ok(events.map(move |event| match event {
            Ok(event) => changes(&context, event),
            Err(e) => Err(Error::of(&context, e)),
        }))
    }
}

fn changes(context: &str, event: WatchEvent<raw::Table>) -> Result<Vec<Change>> {
    let (table, gone) = match event {
        WatchEvent::Added(table) | WatchEvent::Modified(table) => (table, false),
        WatchEvent::Deleted(table) => (table, true),
        WatchEvent::Bookmark(mark) => {
            return Ok(vec![Change::Mark(mark.metadata.resource_version)]);
        }
        WatchEvent::Error(status) => return Err(Error::of(context, kube::Error::Api(status))),
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
