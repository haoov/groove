//! A Table cell that tells a time, and the age as `kubectl` writes it.

use crate::{TableColumn, Timestamp};

/// Server columns that tell a time since an event.
const SINCE: [&str; 3] = ["last schedule", "last seen", "first seen"];

/// One cell of a row that tells a time: it said `said` seconds at `since`, and grows from there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aging {
    pub cell: usize,
    pub since: Timestamp,
    pub said: u64,
    /// The text before the age, the count of `3 (5m ago)`.
    pub lead: Option<String>,
    /// When the cell next reads differently; the epoch until it is first written.
    pub turn: Timestamp,
}

impl Aging {
    /// The cell at `now`, and when it next changes.
    pub fn at(&self, now: Timestamp) -> (String, Timestamp) {
        let seconds = self.said + self.since.age_at(now).as_secs();
        let age = human_age(seconds);
        let text = match &self.lead {
            Some(lead) => format!("{lead} ({age} ago)"),
            None => age,
        };
        let next = Timestamp::new(now.seconds() + next_turn(seconds) as i64);
        (text, next)
    }
}

/// The cells of a row that tell a time, read off the row as it arrived at `received`.
pub fn agings(
    columns: &[TableColumn],
    cells: &[String],
    created: Option<Timestamp>,
    received: Timestamp,
) -> Vec<Aging> {
    let aging = |cell, since, said, lead| Aging {
        cell,
        since,
        said,
        lead,
        turn: Timestamp::default(),
    };
    let mut out = Vec::new();
    for (cell, (column, text)) in columns.iter().zip(cells).enumerate() {
        let name = column.name.to_lowercase();
        match (name.as_str(), created) {
            ("age", Some(created)) => out.push(aging(cell, created, 0, None)),
            ("restarts", _) => {
                let Some((lead, ago)) = text.strip_suffix(" ago)").and_then(|t| t.split_once(" ("))
                else {
                    continue;
                };
                if let Some(said) = crate::cells::read_age(ago) {
                    out.push(aging(cell, received, said, Some(lead.to_string())));
                }
            }
            _ if column.date || name == "age" || SINCE.contains(&name.as_str()) => {
                if let Some(said) = crate::cells::read_age(text) {
                    out.push(aging(cell, received, said, None));
                }
            }
            _ => {}
        }
    }
    out
}

/// `kubectl`'s age: seconds below 2 minutes, minutes and seconds below 10, and so on up to years.
pub fn human_age(seconds: u64) -> String {
    let (minutes, hours) = (seconds / 60, seconds / 3_600);
    let (days, years) = (hours / 24, hours / 8_760);
    match () {
        _ if seconds < 120 => format!("{seconds}s"),
        _ if minutes < 10 => paired(minutes, "m", seconds % 60, "s"),
        _ if minutes < 180 => format!("{minutes}m"),
        _ if hours < 8 => paired(hours, "h", minutes % 60, "m"),
        _ if hours < 48 => format!("{hours}h"),
        _ if hours < 192 => paired(days, "d", hours % 24, "h"),
        _ if hours < 17_520 => format!("{days}d"),
        _ if hours < 70_080 => paired(years, "y", days % 365, "d"),
        _ => format!("{years}y"),
    }
}

fn paired(big: u64, unit: &str, small: u64, under: &str) -> String {
    match small {
        0 => format!("{big}{unit}"),
        _ => format!("{big}{unit}{small}{under}"),
    }
}

/// Seconds until an age of `seconds` reads differently.
fn next_turn(seconds: u64) -> u64 {
    let unit = match seconds {
        0..600 => 1,
        600..28_800 => 60,
        28_800..691_200 => 3_600,
        _ => 86_400,
    };
    unit - seconds % unit
}
