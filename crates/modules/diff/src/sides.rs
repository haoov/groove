//! A file's two sides as text: what the disk holds, what git held, and a stand-in where either is no text.

use std::path::Path;

/// The most a side reads from the disk or from git.
pub const READ_MAX_BYTES: u64 = 16 << 20;

/// What a side shows when it is binary or past `READ_MAX_BYTES`.
pub const UNSHOWN: &str = "Binary, or larger than 16 MB: not shown.\n";

/// The bytes git reads to tell a binary file, as `git diff` does.
const SNIFFED: usize = 8000;

/// The file on disk as text; nothing when it is gone.
pub async fn working(dir: &Path, path: &str) -> String {
    let full = dir.join(path);
    match tokio::fs::metadata(&full).await {
        Ok(meta) if meta.len() > READ_MAX_BYTES => return UNSHOWN.to_string(),
        Ok(_) => {}
        Err(_) => return String::new(),
    }
    match tokio::fs::read(&full).await {
        Ok(bytes) if binary(&bytes) => UNSHOWN.to_string(),
        Ok(bytes) => String::from_utf8(bytes).unwrap_or_else(|_| UNSHOWN.to_string()),
        Err(_) => String::new(),
    }
}

/// A side git read, or the stand-in when it is no text.
pub fn committed(text: &str) -> &str {
    match text.len() as u64 > READ_MAX_BYTES || binary(text.as_bytes()) {
        true => UNSHOWN,
        false => text,
    }
}

/// Both sides, each the stand-in when either is: a file that stands on both sides reads as unchanged.
pub fn paired<'a>(before: &'a str, after: &'a str) -> (&'a str, &'a str) {
    let both = !before.is_empty() && !after.is_empty();
    match both && (before == UNSHOWN || after == UNSHOWN) {
        true => (UNSHOWN, UNSHOWN),
        false => (before, after),
    }
}

fn binary(bytes: &[u8]) -> bool {
    bytes[..bytes.len().min(SNIFFED)].contains(&0)
}
