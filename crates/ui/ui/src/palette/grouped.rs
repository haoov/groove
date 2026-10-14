//! The palette's entries under the query, one run per group, the best group first.

use super::Entry;

/// The order the groups show in while nothing ranks them.
const ORDER: [&str; 10] = [
    "Go", "Session", "Diff", "Git", "MR", "Agent", "Notes", "Task", "Look", "Config",
];

/// The entries the query keeps, grouped; a group the first word names comes first.
pub(super) fn grouped(entries: Vec<Entry>, query: &str) -> Vec<Entry> {
    let mut ranked: Vec<((u8, usize), Entry)> = entries
        .into_iter()
        .filter_map(|entry| Some((rank(&entry, query)?, entry)))
        .collect();
    ranked.sort_by_key(|(key, entry)| (*key, place(entry.group)));
    let mut groups: Vec<&'static str> = Vec::new();
    for (_, entry) in &ranked {
        if !groups.contains(&entry.group) {
            groups.push(entry.group);
        }
    }
    let mut out = Vec::with_capacity(ranked.len());
    for group in groups {
        let (here, rest): (Vec<_>, Vec<_>) = ranked
            .into_iter()
            .partition(|(_, entry)| entry.group == group);
        out.extend(here.into_iter().map(|(_, entry)| entry));
        ranked = rest;
    }
    out
}

/// Where the entry ranks: in a group the query names, then by how early the query matches.
fn rank(entry: &Entry, query: &str) -> Option<(u8, usize)> {
    let mut words = query.split_whitespace();
    if let Some(first) = words.next() {
        let named = entry
            .group
            .to_lowercase()
            .starts_with(&first.to_lowercase());
        let rest = words.collect::<Vec<_>>().join(" ");
        if let Some(score) = named
            .then(|| groove_types::score(&entry.label, &rest))
            .flatten()
        {
            return Some((0, score));
        }
    }
    let text = format!("{} {}", entry.group, entry.label);
    groove_types::score(&text, query).map(|score| (1, score))
}

fn place(group: &str) -> usize {
    ORDER
        .iter()
        .position(|one| *one == group)
        .unwrap_or(ORDER.len())
}
