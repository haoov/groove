//! Markdown prose: parsed into blocks, laid out to a width, drawn with its links.

mod compact;
mod draw;
pub mod layout;
pub mod parse;
mod table;
#[cfg(test)]
mod tests;

use groove_gfx::Rect;

pub use compact::{Row, compact, row};
pub use draw::draw;
pub use layout::{Laid, laid};
pub use parse::blocks;

use crate::base::ctx::{App, Ctx};
use crate::base::style::Role;

/// The first block's text, without its marks.
pub fn plain(text: &str) -> String {
    let blocks = blocks(text).into_iter().map(parse::Block::flattened);
    let first = blocks.into_iter().find(|one| !one.spans.is_empty());
    let words = first.map(|one| one.spans.into_iter().map(|span| span.text));
    words
        .into_iter()
        .flatten()
        .collect::<String>()
        .trim()
        .to_string()
}

/// How tall `text` stands at `width`, as an earlier frame laid it when one did.
pub fn height<A: App>(ctx: &mut Ctx<'_, A>, text: &str, width: f32, role: Role) -> f32 {
    // Kept beside the wrapped rows, apart by a key no text starts with; the value is the height's bits.
    let (key, style) = (format!("\u{0}{text}"), ctx.styles.prose(0, role));
    if let Some(bits) = ctx.kept_rows(&key, &style, width) {
        return f32::from_bits(bits as u32);
    }
    let height = laid(ctx, &blocks(text), width, role).height;
    ctx.keep_rows(&key, &style, width, height.to_bits() as usize);
    height
}

/// `text` from the top of `at`, `role` for its paragraphs; returns the height it stands.
pub fn prose<A: App>(
    ctx: &mut Ctx<'_, A>,
    (at, visible): (Rect, Rect),
    text: &str,
    role: Role,
    link: impl Fn(&str) -> A::Target,
) -> f32 {
    let laid = laid(ctx, &blocks(text), at.w, role);
    draw(ctx, at, visible, &laid, &link);
    laid.height
}
