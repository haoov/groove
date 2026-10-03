//! What a typed query keeps: one rule, wherever something is narrowed by text.

/// Whether `hay` holds every word of `query`, ignoring case.
pub fn narrows(hay: &str, query: &str) -> bool {
    score(hay, query).is_some()
}

/// The same, with how early the words were found, for putting the best first.
pub fn score(hay: &str, query: &str) -> Option<usize> {
    let hay = hay.to_lowercase();
    query
        .split_whitespace()
        .map(|word| hay.find(&word.to_lowercase()))
        .sum()
}

/// Every place `query` stands in `text`, ignoring case, as character ranges of `text` itself.
pub fn occurrences(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    let folded = |s: &str| -> Vec<char> { s.chars().map(fold).collect() };
    let (hay, needle) = (folded(text), folded(query));
    let mut out = Vec::new();
    if needle.is_empty() || needle.len() > hay.len() {
        return out;
    }
    let mut at = 0;
    while at + needle.len() <= hay.len() {
        match hay[at..at + needle.len()] == needle[..] {
            true => {
                out.push(at..at + needle.len());
                at += needle.len();
            }
            false => at += 1,
        }
    }
    out
}

/// One character as a case-blind search compares it.
fn fold(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}
