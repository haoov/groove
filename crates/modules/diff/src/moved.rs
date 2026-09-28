//! Hunks moved along one edit of the new side, before any diff runs again.

use std::ops::Range;

use groove_text::Touched;
use groove_types::Hunk;

/// The hunks once `edit` replaced its lines, and where the one it merged into stands.
pub fn moved(hunks: &[Hunk], edit: Touched) -> (Vec<Hunk>, usize) {
    let span = edit.line..edit.line + edit.gone;
    let touches = |hunk: &Hunk| hunk.after.start < span.end && hunk.after.end > span.start;
    let first = hunks.iter().position(touches);
    let last = hunks.iter().rposition(touches);
    let (before, merged, after) = match (first, last) {
        (Some(first), Some(last)) => (first, merge(&hunks[first..=last], &span), last + 1),
        _ => {
            let at = hunks.partition_point(|hunk| hunk.after.end <= span.start);
            let drift = shift(&hunks[..at]);
            let old = old_of(span.start, drift)..old_of(span.end, drift);
            (
                at,
                Hunk {
                    before: old,
                    after: span.clone(),
                },
                at,
            )
        }
    };
    let grown = i64::from(edit.came) - i64::from(edit.gone);
    let mut out = hunks[..before].to_vec();
    out.push(Hunk {
        before: merged.before,
        after: merged.after.start..moved_by(merged.after.end, grown),
    });
    out.extend(hunks[after..].iter().map(|hunk| Hunk {
        before: hunk.before.clone(),
        after: moved_by(hunk.after.start, grown)..moved_by(hunk.after.end, grown),
    }));
    (out, before)
}

/// The hunks `touched` holds, and the lines of `span` between or around them, as one hunk.
fn merge(touched: &[Hunk], span: &Range<u32>) -> Hunk {
    let (first, last) = (&touched[0], &touched[touched.len() - 1]);
    let old_start = match first.after.start <= span.start {
        true => first.before.start,
        false => first.before.start - (first.after.start - span.start),
    };
    let old_end = match last.after.end >= span.end {
        true => last.before.end,
        false => last.before.end + (span.end - last.after.end),
    };
    Hunk {
        before: old_start..old_end,
        after: first.after.start.min(span.start)..last.after.end.max(span.end),
    }
}

/// How many more new lines than old ones the hunks above a line hold.
fn shift(hunks: &[Hunk]) -> i64 {
    hunks
        .iter()
        .map(|hunk| hunk.after.len() as i64 - hunk.before.len() as i64)
        .sum()
}

/// The old line an unchanged new line stands for.
fn old_of(new: u32, drift: i64) -> u32 {
    u32::try_from(i64::from(new) - drift).unwrap_or(0)
}

fn moved_by(line: u32, grown: i64) -> u32 {
    u32::try_from(i64::from(line) + grown).unwrap_or(0)
}
