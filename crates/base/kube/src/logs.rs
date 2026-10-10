//! A container's log, a line at a time, each with the kubelet's timestamp.

use futures_util::Stream;

use crate::table::encoded;
use crate::{Client, Error, Result};

/// Where a read of the log starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    Lines(u32),
    Seconds(u32),
    /// An RFC 3339 time, to the second: where a broken stream picks up.
    Time(String),
    Beginning,
}

/// One container's log: now, or before its last restart; followed, or read to its end.
#[derive(Debug, Clone, Copy)]
pub struct LogQuery<'a> {
    pub namespace: &'a str,
    pub pod: &'a str,
    pub container: &'a str,
    pub previous: bool,
    pub follow: bool,
    pub start: &'a Start,
}

impl LogQuery<'_> {
    fn uri(&self) -> String {
        let mut pairs = vec![
            ("container", self.container.to_string()),
            ("timestamps", "true".to_string()),
        ];
        if self.previous {
            pairs.push(("previous", "true".to_string()));
        }
        if self.follow {
            pairs.push(("follow", "true".to_string()));
        }
        match self.start {
            Start::Lines(lines) => pairs.push(("tailLines", lines.to_string())),
            Start::Seconds(seconds) => pairs.push(("sinceSeconds", seconds.to_string())),
            Start::Time(at) => pairs.push(("sinceTime", at.clone())),
            Start::Beginning => {}
        }
        let query: Vec<String> = pairs
            .iter()
            .map(|(key, value)| format!("{key}={}", encoded(value)))
            .collect();
        format!(
            "/api/v1/namespaces/{}/pods/{}/log?{}",
            self.namespace,
            self.pod,
            query.join("&")
        )
    }
}

impl Client {
    /// The log's lines as they come; a followed one ends when its container does.
    pub async fn logs(
        &self,
        query: LogQuery<'_>,
    ) -> Result<impl Stream<Item = Result<String>> + use<>> {
        let request = http::Request::get(query.uri())
            .body(Vec::new())
            .map_err(|e| Error::Kubeconfig(e.to_string()))?;
        self.lines(request).await
    }
}
