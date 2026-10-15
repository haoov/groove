//! One entry row of the palette, and where the query lit it.

use std::ops::Range;

use crate::ctx::Ctx;
use crate::hit::Target;
use groove_ui_kit::base::mark::Mark;
use groove_ui_kit::base::style::Role;
use groove_ui_kit::widgets::Row;

pub(super) fn entry<'a>(
    ctx: &Ctx,
    text: &'a str,
    (chord, mark, found): (Option<&'a str>, Option<Mark>, &'a [Range<usize>]),
    selected: bool,
) -> Row<'a, Target> {
    let styles = &ctx.styles;
    let row = Row::new(ctx.tokens.md, text, styles.label(Role::Text));
    let row = row.lit(found, styles.label(Role::Accent));
    let row = match chord {
        Some(chord) => row.end(chord, styles.small(Role::Faint)),
        None => row,
    };
    let tint = styles.color(if selected { Role::Text } else { Role::Faint });
    match mark {
        Some(mark) => row.mark(mark).tint(tint),
        None => row,
    }
}

/// Where the query stands in `text`: fuzzy for a file or a picker's option, else each word.
pub(super) fn matched(text: &str, query: &str, fuzzy: bool) -> Vec<Range<usize>> {
    if let Some(rest) = query.strip_prefix('#') {
        return groove_types::spans(text, &[rest]);
    }
    if fuzzy {
        return groove_types::spans(text, &[query]);
    }
    let mut found: Vec<Range<usize>> = query
        .split_whitespace()
        .filter_map(|word| groove_types::occurrences(text, word).into_iter().next())
        .collect();
    found.sort_by_key(|range| range.start);
    let mut out: Vec<Range<usize>> = Vec::with_capacity(found.len());
    for range in found {
        match out.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => out.push(range),
        }
    }
    out
}
