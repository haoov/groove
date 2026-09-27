//! What `cat-file --batch` writes: a header a line long, then that many bytes.

use std::collections::HashMap;

/// The blobs written, in the order asked; a name that did not resolve is left out.
pub fn batch(out: &[u8], paths: &[&str]) -> HashMap<String, String> {
    let mut found = HashMap::new();
    let mut at = 0;
    for path in paths {
        let Some(line) = out[at..].iter().position(|byte| *byte == b'\n') else {
            break;
        };
        let header = String::from_utf8_lossy(&out[at..at + line]);
        let size = size_of(&header);
        at += line + 1;
        let Some(size) = size else {
            continue;
        };
        let end = (at + size).min(out.len());
        let text = String::from_utf8_lossy(&out[at..end]).into_owned();
        found.insert((*path).to_string(), text);
        at = end + 1;
    }
    found
}

/// The third field of `<sha> <type> <size>`; a missing object has no third field.
fn size_of(header: &str) -> Option<usize> {
    header.split_whitespace().nth(2)?.parse().ok()
}
