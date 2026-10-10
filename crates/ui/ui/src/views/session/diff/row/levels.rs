//! A log line's colours: its clock faint, its container in its own colour, its words by their level.

use groove_types::{Capture, Highlight, Level, level};

/// How wide a line's clock stands, `14:02:09.112`, with the space after it.
const CLOCK: usize = 13;

/// The spans of `text`, which opens with a clock, then a container when `tagged`.
pub(super) fn spans(text: &str, tagged: bool) -> Vec<Highlight> {
    let mut out = Vec::new();
    let mut at = 0;
    if text.len() >= CLOCK {
        out.push(Highlight {
            range: 0..CLOCK - 1,
            capture: Capture::Comment,
        });
        at = CLOCK;
    }
    if tagged && let Some(space) = text.get(at..).and_then(|rest| rest.find(' ')) {
        out.push(Highlight {
            range: at..at + space,
            capture: Capture::Attribute,
        });
        at += space + 1;
    }
    let words = text.get(at..).unwrap_or_default();
    let capture = match level(words) {
        Level::Error => Some(Capture::Error),
        Level::Warn => Some(Capture::Warning),
        Level::Other => None,
    };
    out.extend(capture.map(|capture| Highlight {
        range: at..text.len(),
        capture,
    }));
    out
}
