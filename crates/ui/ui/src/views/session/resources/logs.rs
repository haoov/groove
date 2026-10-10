//! What an object's tab shows, and what its logs view holds: which containers, which run, from where.

use groove_controllers::AppState;
use groove_types::{LogKey, LogRange, LogSource, PodPart};

use super::opened::Opened;

/// Which of the tab's views shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Describe,
    Yaml,
    Logs,
}

impl View {
    /// Whether the editor draws it.
    pub fn edits(self) -> bool {
        matches!(self, View::Yaml | View::Logs)
    }

    pub fn label(self) -> &'static str {
        match self {
            View::Describe => "describe",
            View::Yaml => "yaml",
            View::Logs => "logs",
        }
    }
}

/// The range picker's rows, in the order `LogRange::OFFERED` gives.
pub const RANGES: [&str; 6] = [
    "last 5m",
    "last 15m",
    "last 1h",
    "last 6h",
    "last 24h",
    "from the start",
];

#[derive(Debug, Clone, PartialEq)]
pub struct LogsUi {
    /// The containers read; none is the pod's first that is not an init one.
    pub source: Option<LogSource>,
    pub previous: bool,
    pub range: LogRange,
    /// The view stays on the last line as lines land.
    pub following: bool,
}

impl Default for LogsUi {
    fn default() -> Self {
        Self {
            source: None,
            previous: false,
            range: LogRange::Since(900),
            following: true,
        }
    }
}

/// The stream the tab's logs read, once its pod is known.
pub fn log_key(app: &AppState, tab: &Opened) -> Option<LogKey> {
    let pod = tab.link.read(app)?.pod.as_deref()?;
    let source = match &tab.logs.source {
        Some(one) => one.clone(),
        None => LogSource::Container(first(pod)?),
    };
    Some(LogKey {
        context: tab.link.context.clone(),
        namespace: tab.link.namespace.clone()?,
        pod: tab.link.name.clone(),
        source,
        previous: tab.logs.previous,
        range: tab.logs.range,
    })
}

/// The container a pod's logs open on: its first that is not an init one.
pub fn first(pod: &PodPart) -> Option<String> {
    let running = pod.containers.iter().find(|one| !one.init);
    running
        .or(pod.containers.first())
        .map(|one| one.name.clone())
}
