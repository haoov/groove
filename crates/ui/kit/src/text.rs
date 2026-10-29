//! Text: one line clipped, elided, wrapped, and a short age.

use std::time::Duration;

use groove_gfx::{Rect, TextStyle};

use crate::base::ctx::{App, Ctx};
use crate::base::tokens::ELIDE_SLACK as SLACK;

const ELLIPSIS: char = '\u{2026}';
const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;
const MONTH: u64 = 30 * DAY;
const YEAR: u64 = 365 * DAY;

/// One line of text in `rect`, `indent` from its left edge, clipped to it.
pub fn row<A: App>(ctx: &mut Ctx<'_, A>, rect: Rect, indent: f32, text: &str, style: TextStyle) {
    ctx.clipped(rect, |ctx| {
        ctx.text(text, rect.x + indent, rect.y, rect.h, style);
    });
}

/// A row with the characters in `lit` drawn in `lit_style`, underlined.
pub fn lit_row<A: App>(
    ctx: &mut Ctx<'_, A>,
    (rect, indent): (Rect, f32),
    text: &str,
    style: TextStyle,
    (lit, lit_style): (&[std::ops::Range<usize>], TextStyle),
) {
    let bytes: Vec<usize> = text
        .char_indices()
        .map(|(at, _)| at)
        .chain([text.len()])
        .collect();
    let mut pieces = Vec::new();
    let mut from = 0;
    for range in lit.iter().filter(|range| range.end < bytes.len()) {
        let (start, end) = (bytes[range.start].max(from), bytes[range.end]);
        if start >= end {
            continue;
        }
        pieces.push((from..start, style, false));
        pieces.push((start..end, lit_style, true));
        from = end;
    }
    pieces.push((from..text.len(), style, false));
    ctx.clipped(rect, |ctx| {
        let under = rect.y + (rect.h + style.size) / 2.0;
        for (range, style, lit) in pieces.into_iter().filter(|(range, ..)| !range.is_empty()) {
            let x = rect.x + indent + ctx.measure(&text[..range.start], &style);
            if lit {
                let wide = ctx.measure(&text[range.clone()], &style);
                let line = Rect::new(x, under, wide, ctx.tokens.hairline);
                ctx.quad(line, style.color);
            }
            ctx.text(&text[range], x, rect.y, rect.h, style);
        }
    });
}

/// `text` cut to the room right of `indent`, its `lit` characters drawn in `lit_style` where kept.
pub fn lit_cut<A: App>(
    ctx: &mut Ctx<'_, A>,
    (rect, indent): (Rect, f32),
    text: &str,
    style: TextStyle,
    (lit, lit_style): (&[std::ops::Range<usize>], TextStyle),
) {
    let cut = elide(ctx, text, &style, (rect.w - indent).max(0.0));
    let whole = cut.chars().count() - usize::from(cut != text);
    let kept: Vec<_> = lit.iter().filter(|one| one.end <= whole).cloned().collect();
    lit_row(ctx, (rect, indent), &cut, style, (&kept, lit_style));
}

/// The text as it fits `width`: whole, or cut at a character with an ellipsis.
pub fn elide<A: App>(ctx: &mut Ctx<'_, A>, text: &str, style: &TextStyle, width: f32) -> String {
    let width = width + SLACK;
    if ctx.measure(text, style) <= width {
        return text.to_string();
    }
    let cuts: Vec<usize> = text
        .char_indices()
        .map(|(at, _)| at)
        .chain([text.len()])
        .collect();
    let mut low = 0;
    let mut high = cuts.len() - 1;
    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate = format!("{}{ELLIPSIS}", &text[..cuts[mid]]);
        if ctx.measure(&candidate, style) <= width {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    format!("{}{ELLIPSIS}", text[..cuts[low]].trim_end())
}

/// The text as it fits `width`, cut at the front.
pub fn elide_start<A: App>(
    ctx: &mut Ctx<'_, A>,
    text: &str,
    style: &TextStyle,
    width: f32,
) -> String {
    let width = width + SLACK;
    if ctx.measure(text, style) <= width {
        return text.to_string();
    }
    let cuts: Vec<usize> = text.char_indices().map(|(at, _)| at).collect();
    let mut low = 0;
    let mut high = cuts.len().saturating_sub(1);
    while low < high {
        let mid = (low + high) / 2;
        let candidate = format!("{ELLIPSIS}{}", &text[cuts[mid]..]);
        match ctx.measure(&candidate, style) <= width {
            true => high = mid,
            false => low = mid + 1,
        }
    }
    format!("{ELLIPSIS}{}", &text[cuts[low]..])
}

/// The text as lines that fit `width`, broken at spaces and at its own newlines.
pub fn wrapped<A: App>(
    ctx: &mut Ctx<'_, A>,
    text: &str,
    style: &TextStyle,
    width: f32,
) -> Vec<String> {
    let mut lines = vec![String::new()];
    let mut x = 0.0;
    for word in words(text) {
        let full = x > 0.0 && x + ctx.measure(word.trim_end(), style) > width;
        if word == "\n" || full {
            lines.push(String::new());
            x = 0.0;
        }
        if word == "\n" || (x == 0.0 && word.trim().is_empty()) {
            continue;
        }
        x += ctx.measure(word, style);
        if let Some(line) = lines.last_mut() {
            line.push_str(word);
        }
    }
    for line in &mut lines {
        line.truncate(line.trim_end().len());
    }
    lines
}

/// The text cut after each space, a line break its own word.
pub(crate) fn words(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut from = 0;
    for (at, one) in text.char_indices() {
        if one == '\n' {
            out.extend((from < at).then(|| &text[from..at]));
            out.push("\n");
            from = at + 1;
        } else if one == ' ' {
            out.push(&text[from..=at]);
            from = at + 1;
        }
    }
    out.extend((from < text.len()).then(|| &text[from..]));
    out
}

/// How long ago, in as few characters as it takes: `now`, `2m`, `6h`, `5d`, `3w`, `4mo`, `2y`.
pub fn ago(age: Duration) -> String {
    let seconds = age.as_secs();
    if seconds < MINUTE {
        return "now".to_string();
    }
    let units = [
        (HOUR, MINUTE, "m"),
        (DAY, HOUR, "h"),
        (WEEK, DAY, "d"),
        (MONTH, WEEK, "w"),
        (YEAR, MONTH, "mo"),
    ];
    for (below, each, unit) in units {
        if seconds < below {
            return format!("{}{unit}", seconds / each);
        }
    }
    format!("{}y", seconds / YEAR)
}

/// How long something took: `12s`, `1m 05s`, `2h 03m`.
pub fn lasted(took: Duration) -> String {
    let seconds = took.as_secs();
    match (seconds / HOUR, seconds / MINUTE % MINUTE, seconds % MINUTE) {
        (0, 0, s) => format!("{s}s"),
        (0, m, s) => format!("{m}m {s:02}s"),
        (h, m, _) => format!("{h}h {m:02}m"),
    }
}

/// One line of text, elided to the room it is drawn in.
#[derive(Debug, Clone, Copy)]
pub struct Label<'a> {
    pub text: &'a str,
    pub style: TextStyle,
    pub cut_start: bool,
}

impl<'a> Label<'a> {
    pub fn new(text: &'a str, style: TextStyle) -> Self {
        Self {
            text,
            style,
            cut_start: false,
        }
    }

    pub fn cut_start(mut self) -> Self {
        self.cut_start = true;
        self
    }

    pub fn width<A: App>(&self, ctx: &mut Ctx<'_, A>) -> f32 {
        ctx.measure(self.text, &self.style)
    }

    /// Draws at the left of `room`, which gives up its width and `gap`.
    pub fn left<A: App>(self, ctx: &mut Ctx<'_, A>, room: &mut Rect, gap: f32) -> Rect {
        let width = self.width(ctx).min(room.w);
        let at = room.take_left(width);
        room.take_left(gap);
        self.draw(ctx, at)
    }

    /// Draws at the right of `room`, which gives up its width and `gap`.
    pub fn right<A: App>(self, ctx: &mut Ctx<'_, A>, room: &mut Rect, gap: f32) -> Rect {
        let width = self.width(ctx).min(room.w);
        let at = room.take_right(width);
        room.take_right(gap);
        self.draw(ctx, at)
    }

    /// Draws the text in `rect`; returns the box it took.
    pub fn draw<A: App>(self, ctx: &mut Ctx<'_, A>, rect: Rect) -> Rect {
        let text = match self.cut_start {
            true => elide_start(ctx, self.text, &self.style, rect.w),
            false => elide(ctx, self.text, &self.style, rect.w),
        };
        let width = ctx.measure(&text, &self.style).min(rect.w);
        row(ctx, rect, 0.0, &text, self.style);
        Rect::new(rect.x, rect.y, width, rect.h)
    }
}
