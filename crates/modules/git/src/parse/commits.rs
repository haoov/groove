//! `git log`, as the entries a list shows.

use groove_types::{CommitEntry, Timestamp};

/// The fields one entry asks git for, in this order.
pub const FORMAT: &str = "%H%x1f%h%x1f%an%x1f%at%x1f%s";

/// One record a NUL ends, its fields cut by the unit separator.
pub fn commits(text: &str) -> Vec<CommitEntry> {
    text.split('\0')
        .map(str::trim_start)
        .filter(|record| !record.is_empty())
        .filter_map(entry)
        .collect()
}

fn entry(record: &str) -> Option<CommitEntry> {
    let mut fields = record.split('\u{1f}');
    let sha = fields.next()?.to_string();
    let short_sha = fields.next()?.to_string();
    let author = fields.next()?.to_string();
    let at = fields.next()?.parse().ok().map(Timestamp::new)?;
    let message = fields.next().unwrap_or_default().to_string();
    (!sha.is_empty()).then_some(CommitEntry {
        sha,
        short_sha,
        message,
        author,
        at,
        is_base: false,
    })
}
