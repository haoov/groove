//! A kind's whole objects as JSON, listed then watched; a field or label selector narrows them on the server.

use futures_util::{Stream, StreamExt};
use http::header::ACCEPT;
use serde::Deserialize;
use serde_json::Value;

use crate::table::encoded;
use crate::watch::TIMEOUT;
use crate::{Client, Error, Kind, Result};

const PAGE: &str = "500";
pub(crate) const AS_METADATA: &str =
    "application/json;as=PartialObjectMetadataList;v=v1;g=meta.k8s.io";

/// What a list or a watch of whole objects reads.
#[derive(Debug, Clone, Copy)]
pub struct Narrowed<'a> {
    pub kind: &'a Kind,
    pub namespace: Option<&'a str>,
    /// `metadata.name=api-0`, `involvedObject.uid=…`.
    pub fields: Option<&'a str>,
    pub labels: Option<&'a str>,
}

impl Narrowed<'_> {
    fn uri(&self, extra: &[(&str, &str)]) -> String {
        let mut pairs: Vec<(&str, &str)> = Vec::new();
        if let Some(fields) = self.fields {
            pairs.push(("fieldSelector", fields));
        }
        if let Some(labels) = self.labels {
            pairs.push(("labelSelector", labels));
        }
        pairs.extend_from_slice(extra);
        let query: Vec<String> = pairs
            .iter()
            .map(|(key, value)| format!("{key}={}", encoded(value)))
            .collect();
        let path = self.kind.path(self.namespace);
        match query.is_empty() {
            true => path,
            false => format!("{path}?{}", query.join("&")),
        }
    }
}

/// Every object the list holds, and the version a watch resumes from.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    pub objects: Vec<Value>,
    pub version: String,
}

/// One event of a watch of whole objects.
#[derive(Debug, Clone, PartialEq)]
pub enum Seen {
    Put(Value),
    /// An object gone, as it last stood.
    Gone(Value),
    Mark(String),
}

#[derive(Deserialize)]
struct List {
    #[serde(default)]
    items: Option<Vec<Value>>,
    #[serde(default)]
    metadata: ListMeta,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ListMeta {
    resource_version: Option<String>,
    #[serde(rename = "continue")]
    next: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", content = "object", rename_all = "UPPERCASE")]
enum Event {
    Added(Value),
    Modified(Value),
    Deleted(Value),
    Bookmark(Value),
    Error(Box<kube::core::Status>),
}

impl Client {
    /// Every object `narrowed` names, a page at a time, without `managedFields`.
    pub async fn list_objects(&self, narrowed: Narrowed<'_>) -> Result<Listed> {
        let (mut objects, mut next) = (Vec::new(), None::<String>);
        loop {
            let mut extra = vec![("limit", PAGE)];
            if let Some(token) = next.as_deref() {
                extra.push(("continue", token));
            }
            let list: List = self.json(&narrowed.uri(&extra), "application/json").await?;
            objects.extend(list.items.unwrap_or_default().into_iter().map(bare));
            next = list.metadata.next.filter(|one| !one.is_empty());
            if next.is_none() {
                let version = list.metadata.resource_version.unwrap_or_default();
                return Ok(Listed { objects, version });
            }
        }
    }

    /// The changes since `from`, an event at a time. The stream ends when the server ends it.
    pub async fn watch_objects(
        &self,
        narrowed: Narrowed<'_>,
        from: &str,
    ) -> Result<impl Stream<Item = Result<Seen>> + use<>> {
        let extra = [
            ("watch", "1"),
            ("allowWatchBookmarks", "true"),
            ("resourceVersion", from),
            ("timeoutSeconds", TIMEOUT),
        ];
        let request = http::Request::get(narrowed.uri(&extra))
            .header(ACCEPT, "application/json")
            .body(Vec::new())
            .map_err(|e| Error::Kubeconfig(e.to_string()))?;
        let context = self.context.clone();
        let lines = self.lines(request).await?;
        Ok(lines.map(move |line| line.and_then(|line| seen(&context, &line))))
    }

    /// The metadata of every object `narrowed` names: labels and annotations, never the body.
    pub async fn list_metadata(&self, narrowed: Narrowed<'_>) -> Result<Vec<Value>> {
        let list: List = self.json(&narrowed.uri(&[]), AS_METADATA).await?;
        Ok(list.items.unwrap_or_default())
    }

    /// One object read at `path`; a 404 is `Api` with status 404.
    pub async fn get_object(&self, path: &str) -> Result<Value> {
        self.json(path, "application/json").await
    }

    async fn json<T: serde::de::DeserializeOwned>(&self, uri: &str, accept: &str) -> Result<T> {
        let request = http::Request::get(uri)
            .header(ACCEPT, accept)
            .body(Vec::new())
            .map_err(|e| Error::Kubeconfig(e.to_string()))?;
        self.inner
            .request(request)
            .await
            .map_err(|e| Error::of(&self.context, e))
    }
}

fn seen(context: &str, line: &str) -> Result<Seen> {
    let unreadable = |detail: String| Error::Unreadable {
        context: context.to_string(),
        detail,
    };
    match serde_json::from_str(line).map_err(|e| unreadable(e.to_string()))? {
        Event::Added(object) | Event::Modified(object) => Ok(Seen::Put(bare(object))),
        Event::Deleted(object) => Ok(Seen::Gone(bare(object))),
        Event::Bookmark(object) => {
            let version = object
                .pointer("/metadata/resourceVersion")
                .and_then(Value::as_str);
            Ok(Seen::Mark(version.unwrap_or_default().to_string()))
        }
        Event::Error(status) => Err(Error::of(context, kube::Error::Api(status))),
    }
}

/// The object without its `managedFields`, which nothing in Groove reads.
fn bare(mut object: Value) -> Value {
    if let Some(metadata) = object.get_mut("metadata").and_then(Value::as_object_mut) {
        metadata.remove("managedFields");
    }
    object
}
