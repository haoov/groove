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
