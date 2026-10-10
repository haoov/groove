//! What a log stream reads, each line it brings, and the level a line's words give it.

/// The containers a stream reads: one by name, or every one of the pod.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogSource {
    Container(String),
    All,
}

/// Where a stream starts: the last seconds, or the beginning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogRange {
    Since(u32),
    All,
}

impl LogRange {
    /// The ranges a picker offers, in its order.
    pub const OFFERED: [LogRange; 6] = [
        LogRange::Since(300),
        LogRange::Since(900),
        LogRange::Since(3600),
        LogRange::Since(21_600),
        LogRange::Since(86_400),
        LogRange::All,
    ];
}

/// One stream of logs: a pod's container, or all of them, now or before its last restart.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LogKey {
    pub context: String,
    pub namespace: String,
    pub pod: String,
    pub source: LogSource,
    pub previous: bool,
    pub range: LogRange,
}

/// When a line was written: seconds since the epoch, then nanoseconds.
pub type LogTime = (i64, u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    pub at: Option<LogTime>,
    /// The time as the kubelet wrote it, RFC 3339 in UTC.
    pub stamp: String,
    /// The container it came from, where the stream reads more than one.
    pub container: Option<String>,
    pub text: String,
}

impl LogLine {
    /// A line as the kubelet sends it with timestamps: the time, a space, the text.
    pub fn read(raw: &str, container: Option<String>) -> Self {
        let raw = raw.trim_end_matches('\r');
        let (stamp, text) = raw.split_once(' ').unwrap_or((raw, ""));
        match log_time(stamp) {
            Some(at) => Self {
                at: Some(at),
                stamp: stamp.to_string(),
                container,
                text: text.to_string(),
            },
            None => Self {
                at: None,
                stamp: String::new(),
                container,
                text: raw.to_string(),
            },
        }
    }

    /// `14:02:09.112`: the clock of its time to the millisecond, blank without one.
    pub fn clock(&self) -> String {
        let Some((_, rest)) = self.stamp.split_once('T') else {
            return " ".repeat(12);
        };
        let rest = rest.trim_end_matches('Z');
        let (seconds, fraction) = rest.split_once('.').unwrap_or((rest, ""));
        let millis: String = fraction.chars().chain("000".chars()).take(3).collect();
        format!("{seconds}.{millis}")
    }
}

/// `2026-10-09T14:02:09.112345678Z` read to seconds and nanoseconds.
pub fn log_time(stamp: &str) -> Option<LogTime> {
    let (whole, fraction) = match stamp.trim_end_matches('Z').split_once('.') {
        Some((whole, fraction)) => (format!("{whole}Z"), fraction),
        None => (stamp.to_string(), ""),
    };
    let seconds = crate::Timestamp::parse(&whole).ok()?.seconds();
    if !fraction.chars().all(|one| one.is_ascii_digit()) {
        return None;
    }
    let digits: String = fraction
        .chars()
        .chain("000000000".chars())
        .take(9)
        .collect();
    Some((seconds, digits.parse().ok()?))
}

/// What a line's words say of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Other,
}

/// The level a line names in its first words: `ERROR`, `level=warn`, `"level":"error"`, `E1009`.
pub fn level(text: &str) -> Level {
    let head = text.get(..text.len().min(160)).unwrap_or(text);
    let words = head.split(|one: char| !one.is_ascii_alphanumeric());
    for word in words.filter(|one| !one.is_empty()) {
        let upper = word.to_ascii_uppercase();
        match upper.as_str() {
            "ERROR" | "ERR" | "FATAL" | "PANIC" | "CRITICAL" | "CRIT" | "EMERG" | "ALERT" => {
                return Level::Error;
            }
            "WARN" | "WARNING" => return Level::Warn,
            _ => {}
        }
        if let Some(first) = glog(word) {
            return first;
        }
    }
    Level::Other
}

/// A glog prefix: `E1009`, `W1009`, `F1009`.
fn glog(word: &str) -> Option<Level> {
    let (mark, rest) = word.split_at_checked(1)?;
    if rest.len() != 4 || !rest.chars().all(|one| one.is_ascii_digit()) {
        return None;
    }
    match mark {
        "E" | "F" => Some(Level::Error),
        "W" => Some(Level::Warn),
        _ => None,
    }
}
