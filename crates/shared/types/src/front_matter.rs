//! A file's front matter: `key: value` lines between two `---`, then the words under it.

/// The front matter and what follows it; nothing when the file opens with none.
pub fn split(text: &str) -> Option<(&str, &str)> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---")?;
    let after = &rest[end + 4..];
    let words = after.split_once('\n').map_or("", |(_, words)| words);
    Some((&rest[..end], words))
}

/// One `key: value` line, split on its first colon, quotes around the value taken off.
pub fn field(front: &str, key: &str) -> Option<String> {
    front.lines().find_map(|line| {
        let (named, value) = line.split_once(':')?;
        let value = value.trim().trim_matches(['"', '\'']).trim();
        (named.trim() == key && !value.is_empty()).then(|| value.to_string())
    })
}

/// A comma-separated value as its items, the empty ones left out.
pub fn listed(said: Option<&str>) -> Vec<String> {
    let items = said.into_iter().flat_map(|one| one.split(','));
    let items = items.map(|one| one.trim().to_string());
    items.filter(|one| !one.is_empty()).collect()
}
