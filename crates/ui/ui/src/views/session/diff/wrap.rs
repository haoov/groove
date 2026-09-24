//! A note's words laid out on rows as wide as the surface gives them.

use groove_types::Note;

use crate::Ui;

/// How many characters a note row holds before the frame has said.
const COLS: usize = 80;

/// The fewest a row is ever given, so a narrow pane still reads.
const FEWEST: usize = 20;

/// How many characters a note row holds, as the last frame measured it.
pub(crate) fn cols_of(ui: &Ui) -> usize {
    match ui.session.note_cols {
        0 => COLS,
        cols => cols.max(FEWEST),
    }
}

/// Every row a note takes: who said it on each reply's first row, then its words.
pub(crate) fn wrapped(note: &Note, cols: usize) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for said in &note.said {
        for (at, line) in wrap(&said.body, cols).into_iter().enumerate() {
            let author = match at {
                0 => said.author.clone(),
                _ => String::new(),
            };
            out.push((author, line));
        }
    }
    out
}

/// A body broken between words, each of its own lines kept.
pub(crate) fn wrap(body: &str, cols: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        let mut current = String::new();
        for word in line.split_whitespace() {
            placed(&mut out, &mut current, word, cols);
        }
        out.push(current);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// One word onto the row, a new row when it does not fit, cut when it is wider than one.
fn placed(out: &mut Vec<String>, current: &mut String, word: &str, cols: usize) {
    let taken = current.chars().count();
    if taken > 0 && taken + 1 + word.chars().count() > cols {
        out.push(std::mem::take(current));
    }
    let mut word = word;
    while word.chars().count() > cols {
        if !current.is_empty() {
            out.push(std::mem::take(current));
        }
        let cut = word
            .char_indices()
            .nth(cols)
            .map_or(word.len(), |(at, _)| at);
        out.push(word[..cut].to_string());
        word = &word[cut..];
    }
    if !current.is_empty() {
        current.push(' ');
    }
    current.push_str(word);
}
