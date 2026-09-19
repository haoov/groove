use groove_gfx::{Color, Rect, TextStyle};

use crate::ctx::Ctx;

const ELLIPSIS: char = '\u{2026}';

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

/// The text as it fits `width`, cut at the front so its end survives.
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

/// A hairline above `rect` and one below it.
pub fn ruled(ctx: &mut Ctx, rect: Rect, color: Color) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(Rect::new(rect.x, rect.y, rect.w, thickness), color);
    let under = rect.bottom() - thickness;
    ctx.quad(Rect::new(rect.x, under, rect.w, thickness), color);
}

/// A hairline along the bottom of `rect`.
pub fn hairline(ctx: &mut Ctx, rect: Rect, color: Color) {
    let thickness = ctx.tokens.hairline;
    ctx.quad(
        Rect::new(rect.x, rect.bottom() - thickness, rect.w, thickness),
        color,
    );
}
