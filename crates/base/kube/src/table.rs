//! A kind's objects as the server's Table: its columns, and a row of cells an object.

use http::header::ACCEPT;

use crate::{Client, Error, Kind, Result};

/// The Table, with each row's own metadata and nothing else of the object.
pub(crate) const AS_TABLE: &str = "application/json;as=Table;v=v1;g=meta.k8s.io,application/json";
const PAGE: usize = 5_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    /// 0 for the columns `kubectl get` shows; above for `-o wide`.
    pub priority: i32,
    /// The server writes the cell as an age.
    pub date: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub uid: String,
    pub name: String,
    pub namespace: Option<String>,
    pub version: String,
    pub created: Option<String>,
    pub cells: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
    /// The list's resourceVersion, which a watch resumes from.
    pub version: String,
    /// The token of the next page, while there is one.
    pub next: Option<String>,
}

/// What a list or a watch reads: a kind, in one namespace or all, narrowed by a label selector.
#[derive(Debug, Clone, Copy)]
pub struct Query<'a> {
    pub kind: &'a Kind,
    pub namespace: Option<&'a str>,
    pub selector: Option<&'a str>,
}

impl Query<'_> {
    /// The path and query string, with `extra` pairs after the selector.
    pub(crate) fn uri(&self, extra: &[(&str, &str)]) -> String {
        let mut pairs: Vec<(&str, &str)> = vec![("includeObject", "Metadata")];
        if let Some(selector) = self.selector {
            pairs.push(("labelSelector", selector));
        }
        pairs.extend_from_slice(extra);
        let query: Vec<String> = pairs
            .iter()
            .map(|(key, value)| format!("{key}={}", encoded(value)))
            .collect();
        format!("{}?{}", self.kind.path(self.namespace), query.join("&"))
    }
}

impl Client {
    /// One page of the list: the first from the API server's cache, not etcd; then from `next`.
    pub async fn page(&self, query: Query<'_>, next: Option<&str>) -> Result<Page> {
        let limit = PAGE.to_string();
        let mut extra = vec![("limit", limit.as_str())];
        match next {
            Some(next) => extra.push(("continue", next)),
            None => extra.extend([
                ("resourceVersion", "0"),
                ("resourceVersionMatch", "NotOlderThan"),
            ]),
        }
        let request = http::Request::get(query.uri(&extra))
            .header(ACCEPT, AS_TABLE)
            .body(Vec::new())
            .map_err(|e| Error::Kubeconfig(e.to_string()))?;
        let raw: raw::Table = self
            .inner
            .request(request)
            .await
            .map_err(|e| Error::of(&self.context, e))?;
        Ok(raw.into())
    }
}

/// Every byte outside the unreserved set, percent-encoded.
pub(crate) fn encoded(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The Table as the server writes it; every field Groove does not read is skipped.
pub(crate) mod raw {
    use serde::{Deserialize, Deserializer};

    /// A list the server may write as `null`, read as empty.
    fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Vec<T>, D::Error> {
        Ok(Option::<Vec<T>>::deserialize(d)?.unwrap_or_default())
    }

    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct Table {
        #[serde(default, rename = "columnDefinitions", deserialize_with = "nullable")]
        pub columns: Vec<Column>,
        #[serde(default, deserialize_with = "nullable")]
        pub rows: Vec<Row>,
        #[serde(default)]
        pub metadata: Meta,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Column {
        pub name: String,
        #[serde(default)]
        pub priority: i32,
        #[serde(default, rename = "type")]
        pub kind: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Row {
        #[serde(default, deserialize_with = "nullable")]
        pub cells: Vec<serde_json::Value>,
        #[serde(default)]
        pub object: Object,
    }

    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct Object {
        #[serde(default)]
        pub metadata: Meta,
    }

    #[derive(Debug, Clone, Default, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Meta {
        pub uid: Option<String>,
        pub name: Option<String>,
        pub namespace: Option<String>,
        pub resource_version: Option<String>,
        pub creation_timestamp: Option<String>,
        #[serde(rename = "continue")]
        pub next: Option<String>,
    }
}

impl From<raw::Table> for Page {
    fn from(raw: raw::Table) -> Self {
        Page {
            columns: raw.columns.into_iter().map(Column::from).collect(),
            rows: raw.rows.into_iter().map(Row::from).collect(),
            version: raw.metadata.resource_version.unwrap_or_default(),
            next: raw.metadata.next.filter(|one| !one.is_empty()),
        }
    }
}

impl From<raw::Column> for Column {
    fn from(raw: raw::Column) -> Self {
        Column {
            name: raw.name,
            priority: raw.priority,
            date: raw.kind == "date",
        }
    }
}

impl From<raw::Row> for Row {
    fn from(raw: raw::Row) -> Self {
        let meta = raw.object.metadata;
        Row {
            uid: meta.uid.unwrap_or_default(),
            name: meta.name.unwrap_or_default(),
            namespace: meta.namespace,
            version: meta.resource_version.unwrap_or_default(),
            created: meta.creation_timestamp,
            cells: raw.cells.iter().map(cell).collect(),
        }
    }
}

/// A cell as it reads: a string as is, a number or a flag in its JSON form, null empty.
fn cell(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
