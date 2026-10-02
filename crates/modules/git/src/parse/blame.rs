//! `git blame --porcelain`, as one entry per line of the file.

use std::collections::HashMap;

use groove_types::{BlameLine, Timestamp};

/// How many characters of a sha the blame shows.
const SHORT: usize = 7;

/// What porcelain prints once per commit, for that commit's later lines.
#[derive(Clone, Default)]
struct Meta {
    author: String,
    at: i64,
    summary: String,
}

/// Every line of the blamed file, in order.
pub fn blame(text: &str) -> Vec<BlameLine> {
    let mut known: HashMap<String, Meta> = HashMap::new();
    let mut lines = Vec::new();
    let (mut sha, mut line, mut meta) = (String::new(), 0, Meta::default());
    for raw in text.lines() {
        if raw.starts_with('\t') {
            if !meta.author.is_empty() {
                known.insert(sha.clone(), std::mem::take(&mut meta));
            }
            let seen = known.get(&sha).cloned().unwrap_or_default();
            lines.push(BlameLine {
                line,
                short_sha: sha.chars().take(SHORT).collect(),
                author: seen.author,
                at: Timestamp::new(seen.at),
                summary: seen.summary,
                uncommitted: sha.bytes().all(|c| c == b'0'),
                sha: sha.clone(),
            });
            continue;
        }
        if let Some(rest) = raw.strip_prefix("author ") {
            meta.author = rest.to_string();
        } else if let Some(rest) = raw.strip_prefix("author-time ") {
            meta.at = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = raw.strip_prefix("summary ") {
            meta.summary = rest.to_string();
        } else if let Some((at, final_line)) = header(raw) {
            sha = at.to_string();
            line = final_line;
        }
    }
    lines.sort_by_key(|one| one.line);
    lines
}

/// A group's first line: the sha, then the original and the final line from 1.
fn header(raw: &str) -> Option<(&str, u32)> {
    let bytes = raw.as_bytes();
    let hex =
        bytes.len() > 40 && bytes[40] == b' ' && bytes[..40].iter().all(u8::is_ascii_hexdigit);
    if !hex {
        return None;
    }
    let mut parts = raw.split(' ');
    let sha = parts.next()?;
    let final_line: u32 = parts.nth(1)?.parse().ok()?;
    Some((sha, final_line.checked_sub(1)?))
}
