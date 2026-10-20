//! What a character belongs to, for picking out a word.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Word,
    Space,
    Mark,
}

/// Whether `c` belongs to a word: a letter, a digit or an underscore.
pub fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn class(c: char) -> Class {
    match c {
        _ if is_word(c) => Class::Word,
        _ if c.is_whitespace() => Class::Space,
        _ => Class::Mark,
    }
}

/// The column the word ending at `column` of `line` starts at.
pub fn word_start(line: &str, column: usize) -> usize {
    let before: Vec<char> = line.chars().take(column).collect();
    let word = before.iter().rev().take_while(|c| is_word(**c)).count();
    before.len() - word
}

/// The word around `column` of `line`, the column at its start, inside it or at its end.
pub fn word_around(line: &str, column: usize) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    let column = column.min(chars.len());
    let before = chars[..column]
        .iter()
        .rev()
        .take_while(|c| is_word(**c))
        .count();
    let after = chars[column..].iter().take_while(|c| is_word(**c)).count();
    let word: String = chars[column - before..column + after].iter().collect();
    (!word.is_empty()).then_some(word)
}

/// Where `word` stands whole in `line`, in characters.
pub fn whole_words(line: &str, word: &str) -> Vec<std::ops::Range<usize>> {
    let chars: Vec<char> = line.chars().collect();
    let sought: Vec<char> = word.chars().collect();
    if sought.is_empty() {
        return Vec::new();
    }
    let part = |at: Option<&char>| at.is_some_and(|c| is_word(*c));
    (0..chars.len().saturating_sub(sought.len() - 1))
        .filter(|at| {
            chars[*at..].starts_with(&sought)
                && !part(at.checked_sub(1).and_then(|before| chars.get(before)))
                && !part(chars.get(at + sought.len()))
        })
        .map(|at| at..at + sought.len())
        .collect()
}
