//! Text: one line clipped, elided, wrapped, and a short age.

use std::time::Duration;

use groove_gfx::{Rect, TextStyle};

use crate::base::ctx::Ctx;

const ELLIPSIS: char = '\u{2026}';
const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;

/// One line of text in `rect`, `indent` from its left edge, clipped to it.
pub fn row(ctx: &mut Ctx, rect: Rect, indent: f32, text: &str, style: TextStyle) {
    ctx.clipped(rect, |ctx| {
        ctx.text(text, rect.x + indent, rect.y, rect.h, style);
    });
}

/// The text as it fits `width`: whole, or cut at a character with an ellipsis.
pub fn elide(ctx: &mut Ctx, text: &str, style: &TextStyle, width: f32) -> String {
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
pub fn elide_start(ctx: &mut Ctx, text: &str, style: &TextStyle, width: f32) -> String {
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
pub fn wrapped(ctx: &mut Ctx, text: &str, style: &TextStyle, width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = match line.is_empty() {
                true => word.to_string(),
                false => format!("{line} {word}"),
            };
            if ctx.measure(&candidate, style) > width && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
                continue;
            }
            line = candidate;
        }
        lines.push(line);
    }
    lines
}

/// How long ago, in as few characters as it takes: `now`, `2m`, `6h`, `5d`.
pub fn ago(age: Duration) -> String {
    let seconds = age.as_secs();
    if seconds < MINUTE {
        return "now".to_string();
    }
    if seconds < HOUR {
        return format!("{}m", seconds / MINUTE);
    }
    if seconds < DAY {
        return format!("{}h", seconds / HOUR);
    }
    format!("{}d", seconds / DAY)
}

/// One line of text, elided to the room it is drawn in.
#[derive(Debug, Clone, Copy)]
pub struct Label<'a> {
    pub text: &'a str,
    pub style: TextStyle,
}

impl<'a> Label<'a> {
    pub fn new(text: &'a str, style: TextStyle) -> Self {
        Self { text, style }
    }

    pub fn width(&self, ctx: &mut Ctx) -> f32 {
        ctx.measure(self.text, &self.style)
    }

    /// Draws at the left of `room`, which gives up its width and `gap`.
    pub fn left(self, ctx: &mut Ctx, room: &mut Rect, gap: f32) -> Rect {
        let width = self.width(ctx).min(room.w);
        let at = room.take_left(width);
        room.take_left(gap);
        self.draw(ctx, at)
    }

    /// Draws at the right of `room`, which gives up its width and `gap`.
    pub fn right(self, ctx: &mut Ctx, room: &mut Rect, gap: f32) -> Rect {
        let width = self.width(ctx).min(room.w);
        let at = room.take_right(width);
        room.take_right(gap);
        self.draw(ctx, at)
    }

    /// Draws the text in `rect`; returns the box it took.
    pub fn draw(self, ctx: &mut Ctx, rect: Rect) -> Rect {
        let text = elide(ctx, self.text, &self.style, rect.w);
        let width = ctx.measure(&text, &self.style).min(rect.w);
        row(ctx, rect, 0.0, &text, self.style);
        Rect::new(rect.x, rect.y, width, rect.h)
    }
}
