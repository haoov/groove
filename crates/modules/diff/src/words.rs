//! What two lines of a pair say differently, in the columns each one draws.

use std::ops::Range;

use groove_text::{Class, class};
use imara_diff::{Algorithm, Diff, InternedInput, Interner, Token};

/// Longer than this a line is marked whole.
const MAX_CHARS: usize = 400;

/// A pair keeping less than this of its longer line is a rewrite, not an edit.
const KEPT_MIN: f32 = 0.25;

/// The columns of `before` that `after` does not have, and the other way round.
pub fn between(before: &str, after: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let longest = before.chars().count().max(after.chars().count());
    if longest > MAX_CHARS {
        return (Vec::new(), Vec::new());
    }
    let (old, new) = (words(before), words(after));
    let input = interned(&old, &new);
    let diff = Diff::compute(Algorithm::Histogram, &input);
    let mut changed = (Vec::new(), Vec::new());
    let mut kept = 0;
    let mut at = (0, 0);
    for hunk in diff.hunks() {
        kept += held(&old, at.0..hunk.before.start as usize);
        push(&mut changed.0, &old, hunk.before.clone());
        push(&mut changed.1, &new, hunk.after.clone());
        at = (hunk.before.end as usize, hunk.after.end as usize);
    }
    kept += held(&old, at.0..old.len());
    match (kept as f32) < longest as f32 * KEPT_MIN {
        true => (Vec::new(), Vec::new()),
        false => changed,
    }
}

/// The characters a run of words covers.
fn held(words: &[Word<'_>], run: Range<usize>) -> usize {
    words[run].iter().map(|word| word.at.len()).sum()
}

/// One hunk's words as the columns they cover.
fn push(ranges: &mut Vec<Range<usize>>, words: &[Word<'_>], run: Range<u32>) {
    let run = run.start as usize..run.end as usize;
    if run.is_empty() {
        return;
    }
    let (Some(first), Some(last)) = (words.get(run.start), words.get(run.end - 1)) else {
        return;
    };
    ranges.push(first.at.start..last.at.end);
}

/// One word: the columns it covers, and what it says.
struct Word<'a> {
    at: Range<usize>,
    text: &'a str,
}

/// A line cut into runs of one kind; every mark is a run of its own.
fn words(line: &str) -> Vec<Word<'_>> {
    let mut words = Vec::new();
    let mut from = (0, 0);
    let mut kind = None;
    let mut chars = 0;
    for (at, (byte, c)) in line.char_indices().enumerate() {
        chars = at + 1;
        let class = class(c);
        if kind == Some(class) && class != Class::Mark {
            continue;
        }
        if kind.is_some() {
            words.push(Word {
                at: from.0..at,
                text: &line[from.1..byte],
            });
        }
        from = (at, byte);
        kind = Some(class);
    }
    if kind.is_some() {
        words.push(Word {
            at: from.0..chars,
            text: &line[from.1..],
        });
    }
    words
}

fn interned<'a>(old: &'a [Word<'a>], new: &'a [Word<'a>]) -> InternedInput<&'a str> {
    let mut interner = Interner::new(old.len() + new.len());
    let before = tokens(old, &mut interner);
    let after = tokens(new, &mut interner);
    InternedInput {
        before,
        after,
        interner,
    }
}

fn tokens<'a>(words: &'a [Word<'a>], interner: &mut Interner<&'a str>) -> Vec<Token> {
    words
        .iter()
        .map(|word| interner.intern(word.text))
        .collect()
}
